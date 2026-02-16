use serde_json::{json, Value};
use worker::*;

const CLAUDE_CODE_SYSTEM_PROMPT: &str =
    "You are Claude Code, Anthropic's official CLI for Claude.";

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    if !req.path().starts_with("/v1") {
        let resp = Response::from_json(&json!({"msg": "not found"}))?;
        return Ok(resp.with_status(404));
    }

    let upstream_url = env.var("UPSTREAM_URL")?.to_string();
    console_log!("[{}] {} {}", req.method(), req.path(), upstream_url);
    match proxy_request(req, &upstream_url).await {
        Ok(resp) => {
            console_log!("-> upstream responded {}", resp.status_code());
            Ok(resp)
        }
        Err(e) => {
            console_error!("proxy error: {}", e);
            Response::error(format!("proxy error: {}", e), 502)
        }
    }
}

async fn proxy_request(mut req: Request, upstream_url: &str) -> Result<Response> {
    let url = req.url()?;
    let upstream = match url.query() {
        Some(q) => format!("{}{}?{}", upstream_url, url.path(), q),
        None => format!("{}{}", upstream_url, url.path()),
    };

    let method = req.method();
    let has_body = method == Method::Post || method == Method::Put || method == Method::Patch;

    let headers = Headers::new();
    for (name, value) in req.headers() {
        let lower = name.to_lowercase();
        if lower == "host" || lower == "connection" {
            continue;
        }
        headers.set(&name, &value)?;
    }

    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);

    if has_body {
        let body_bytes = req.bytes().await?;
        console_log!("request body size: {} bytes", body_bytes.len());
        let modified_body = inject_system_prompt(&body_bytes);
        console_log!("modified body size: {} bytes", modified_body.len());
        init.with_body(Some(modified_body.into()));
    }

    let proxy_req = Request::new_with_init(&upstream, &init)?;
    Fetch::Request(proxy_req).send().await
}

fn inject_system_prompt(body: &[u8]) -> Vec<u8> {
    let mut json: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return body.to_vec(),
    };

    let prompt_entry = json!({
        "type": "text",
        "text": CLAUDE_CODE_SYSTEM_PROMPT
    });

    let new_system = match json.get("system") {
        Some(Value::Array(arr)) => {
            let mut merged = vec![prompt_entry];
            merged.extend(arr.iter().cloned());
            Value::Array(merged)
        }
        _ => {
            Value::Array(vec![prompt_entry])
        }
    };

    json["system"] = new_system;

    serde_json::to_vec(&json).unwrap_or_else(|_| body.to_vec())
}
