use worker::*;

/// Pure bearer parsing shared by production and tests.
pub fn parse_bearer_header(header: Option<&str>) -> Option<String> {
    header
        .and_then(|value| value.strip_prefix("Bearer ").map(str::to_string))
        .filter(|token| !token.is_empty())
}

pub fn json_error(message: &str, status: u16) -> Result<Response> {
    Ok(Response::from_json(&serde_json::json!({ "error": message }))?.with_status(status))
}

pub fn json_ok<T: serde::Serialize>(data: &T) -> Result<Response> {
    Response::from_json(data)
}


