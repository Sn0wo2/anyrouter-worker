pub fn is_proxy_path(path: &str) -> bool {
    path.starts_with("/v1")
}

pub fn build_upstream_url(upstream_base: &str, path: &str, query: Option<&str>) -> String {
    match query {
        Some(q) => format!("{}{}?{}", upstream_base, path, q),
        None => format!("{}{}", upstream_base, path),
    }
}

pub fn should_have_body(method: &str) -> bool {
    matches!(method, "POST" | "PUT" | "PATCH")
}

pub fn should_forward_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower != "host" && lower != "connection"
}
