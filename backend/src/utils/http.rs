use worker::*;

/// Pure bearer parsing shared by production and tests.
pub fn parse_bearer_header(header: Option<&str>) -> Option<String> {
    header
        .and_then(|value| value.strip_prefix("Bearer ").map(str::to_string))
        .filter(|token| !token.is_empty())
}

/// Reads one cookie value from a `Cookie` header. Pure.
pub fn parse_cookie_header(header: Option<&str>, name: &str) -> Option<String> {
    let header = header?;
    for part in header.split(';') {
        if let Some((key, value)) = part.trim().split_once('=') {
            if key.trim() == name {
                let value = value.trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Builds the session cookie set at login for the hub WebSocket.
/// `secure` adds `Secure; SameSite=None` (cross-site production);
/// otherwise `SameSite=Lax` (same-site, including local dev).
pub fn build_session_cookie(name: &str, token: &str, max_age_secs: u64, secure: bool) -> String {
    if secure {
        format!("{name}={token}; Path=/api; HttpOnly; Secure; SameSite=None; Max-Age={max_age_secs}")
    } else {
        format!("{name}={token}; Path=/api; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}")
    }
}

/// Builds the expired session cookie that clears the login cookie.
pub fn build_expired_session_cookie(name: &str, secure: bool) -> String {
    if secure {
        format!("{name}=; Path=/api; HttpOnly; Secure; SameSite=None; Max-Age=0")
    } else {
        format!("{name}=; Path=/api; HttpOnly; SameSite=Lax; Max-Age=0")
    }
}

pub fn json_error(message: &str, status: u16) -> Result<Response> {
    Ok(Response::from_json(&serde_json::json!({ "error": message }))?.with_status(status))
}

pub fn json_ok<T: serde::Serialize>(data: &T) -> Result<Response> {
    Response::from_json(data)
}


