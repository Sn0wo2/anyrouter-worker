use serde_json::json;
use worker::*;

mod constants;
mod payload;
mod proxy_core;

use payload::patch_request_body;
use proxy_core::{build_upstream_url, is_proxy_path, should_forward_header, should_have_body};

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    if !is_proxy_path(&req.path()) {
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
    let upstream = build_upstream_url(upstream_url, url.path(), url.query());

    let method = req.method();
    let has_body = should_have_body(method.as_ref());

    let headers = Headers::new();
    for (name, value) in req.headers() {
        if !should_forward_header(&name) {
            continue;
        }
        headers.set(&name, &value)?;
    }

    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);

    if has_body {
        let body_bytes = req.bytes().await?;
        // console_log!("=== ORIGINAL BODY ===");
        // console_log!("{}", String::from_utf8_lossy(&body_bytes));

        let modified_body = patch_request_body(&body_bytes);
        // console_log!("=== MODIFIED BODY ===");
        // console_log!("{}", String::from_utf8_lossy(&modified_body));

        init.with_body(Some(modified_body.into()));
    }

    let proxy_req = Request::new_with_init(&upstream, &init)?;
    Fetch::Request(proxy_req).send().await
}
