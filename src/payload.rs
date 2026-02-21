use serde_json::{json, Value};

use crate::constants::{
    CLAUDE_CODE_SYSTEM_FALLBACK, CLAUDE_CODE_SYSTEM_PROMPT, DEFAULT_THINKING_BUDGET,
};

pub fn patch_request_body(body: &[u8]) -> Vec<u8> {
    let mut json: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return body.to_vec(),
    };

    if let Some(root) = json.as_object_mut() {
        ensure_system(root);
        ensure_thinking(root);
    }

    serde_json::to_vec(&json).unwrap_or_else(|_| body.to_vec())
}

fn ensure_system(root: &mut serde_json::Map<String, Value>) {
    let prompt_entry = json!({
        "type": "text",
        "text": CLAUDE_CODE_SYSTEM_PROMPT
    });
    let fallback_entry = json!({
        "type": "text",
        "text": CLAUDE_CODE_SYSTEM_FALLBACK
    });

    match root.remove("system") {
        Some(Value::Array(mut arr)) => {
            if arr.first().and_then(system_text) != Some(CLAUDE_CODE_SYSTEM_PROMPT) {
                arr.insert(0, prompt_entry);
            }
            if arr.len() < 2 {
                arr.push(fallback_entry);
            }
            if let Some(first) = arr.get_mut(0) {
                normalize_system_entry_in_place(first);
            }
            if let Some(second) = arr.get_mut(1) {
                normalize_system_entry_in_place(second);
            }
            root.insert("system".to_string(), Value::Array(arr));
        }
        Some(Value::String(s)) => {
            let mut arr = Vec::with_capacity(2);
            arr.push(prompt_entry);
            if s == CLAUDE_CODE_SYSTEM_PROMPT {
                arr.push(fallback_entry);
            } else {
                arr.push(json!({"type": "text", "text": s}));
            }
            root.insert("system".to_string(), Value::Array(arr));
        }
        Some(other) => {
            root.insert("system".to_string(), other);
        }
        None => {
            root.insert(
                "system".to_string(),
                Value::Array(vec![prompt_entry, fallback_entry]),
            );
        }
    }
}

fn ensure_thinking(root: &mut serde_json::Map<String, Value>) {
    if root.contains_key("thinking") {
        return;
    }

    let max_tokens = root
        .get("max_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_THINKING_BUDGET + 1);
    let budget_tokens = std::cmp::min(DEFAULT_THINKING_BUDGET, max_tokens.saturating_sub(1)).max(1);

    root.insert(
        "thinking".to_string(),
        json!({
            "type": "enabled",
            "budget_tokens": budget_tokens
        }),
    );
}

fn normalize_system_entry_in_place(value: &mut Value) {
    match value {
        Value::String(s) => {
            let text = std::mem::take(s);
            *value = json!({"type": "text", "text": text});
        }
        Value::Object(map) => {
            if map.get("text").and_then(Value::as_str).is_some() && !map.contains_key("type") {
                map.insert("type".to_string(), json!("text"));
            }
        }
        _ => {}
    }
}

fn system_text(value: &Value) -> Option<&str> {
    match value {
        Value::Object(map) => map.get("text").and_then(Value::as_str),
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}
