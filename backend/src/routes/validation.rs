use crate::common::DeviceLifecycleKind;
use crate::models::device::{AuthorizeDeviceRequest, CheckSessionRequest, CreateSessionRequest};
use crate::models::auth::GoogleAuthRequest;
use crate::models::error::AppError;

/// Mirrors `routes::auth::google_auth` credential validation.
pub fn require_credential(body: &GoogleAuthRequest) -> Result<String, AppError> {
    match body.credential.as_deref().map(str::trim) {
        Some(c) if !c.is_empty() => Ok(c.to_string()),
        _ => Err(AppError::bad_request(
            "Missing 'credential' (Google ID token).",
        )),
    }
}

/// Mirrors `routes::device::create_session` session_id validation.
pub fn require_session_id(body: &CreateSessionRequest) -> Result<String, AppError> {
    match body.session_id.as_deref().map(str::trim) {
        Some(id) if !id.is_empty() => Ok(id.to_string()),
        _ => Err(AppError::bad_request("Missing 'session_id'.")),
    }
}

/// Mirrors `routes::device::authorize_device` code/session resolution.
pub fn resolve_code_or_session(body: &AuthorizeDeviceRequest) -> Result<String, AppError> {
    let code_or_session = body
        .session_id
        .as_deref()
        .or(body.code.as_deref())
        .unwrap_or("")
        .trim();
    if code_or_session.is_empty() {
        return Err(AppError::bad_request("Missing 'session_id' or 'code'."));
    }
    Ok(code_or_session.to_string())
}

/// Mirrors `routes::device::check_session` session_id validation.
pub fn require_check_session_id(body: &CheckSessionRequest) -> Result<String, AppError> {
    match body.session_id.as_deref().map(str::trim) {
        Some(id) if !id.is_empty() => Ok(id.to_string()),
        _ => Err(AppError::bad_request("Missing 'session_id'.")),
    }
}

/// Pure version of bearer extraction for testing (takes the header value).
/// Shares semantics with `utils::parse_bearer_header`.
pub fn require_bearer_header(header: Option<&str>) -> Result<String, AppError> {
    match crate::utils::parse_bearer_header(header) {
        Some(t) => Ok(t),
        None => Err(AppError::unauthorized(
            "Missing or invalid Authorization header.",
        )),
    }
}

/// Mirrors `routes::device::revoke_device` token param validation.
pub fn require_route_token(param: Option<&str>) -> Result<String, AppError> {
    match param {
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err(AppError::bad_request("Missing device token.")),
    }
}

/// Mirrors `routes::auth::get_user` sub-match check.
pub fn require_sub_match(profile_sub: &str, route_sub: &str) -> Result<(), AppError> {
    if profile_sub != route_sub {
        return Err(AppError::forbidden("Forbidden."));
    }
    Ok(())
}

/// Mirrors `routes::auth::get_user` missing-sub check.
pub fn require_route_sub(param: Option<&str>) -> Result<String, AppError> {
    match param {
        Some(s) if !s.is_empty() => Ok(s.to_string()),
        _ => Err(AppError::bad_request("Missing user id.")),
    }
}

/// Distinguishes mobile self-disconnect from web-dashboard revocation.
/// When the caller's auth token matches the token to revoke, the device
/// is disconnecting itself.
pub fn resolve_lifecycle_kind(auth_token: &str, token_to_revoke: &str) -> DeviceLifecycleKind {
    if auth_token == token_to_revoke {
        DeviceLifecycleKind::Disconnected
    } else {
        DeviceLifecycleKind::Revoked
    }
}

