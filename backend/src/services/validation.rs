use crate::common::{DEFAULT_DEVICE_NAME, SHORT_CODE_LEN};
use crate::models::error::{AppError, AppResult};
use crate::utils::is_valid_128_hex;

/// Validates a raw session id: trims and enforces 128-hex shape.
/// Mirrors `DeviceService::create_session` validation exactly.
pub fn validate_session_id(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if !is_valid_128_hex(trimmed) {
        return Err(AppError::bad_request(
            "Invalid session_id. Must be a 128-character hex string.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Derives the lowercase short code from a validated session id.
pub fn derive_short_code(validated_session_id: &str) -> String {
    validated_session_id[..SHORT_CODE_LEN].to_lowercase()
}

/// Formats a stored short code for API responses (uppercase).
pub fn present_short_code(stored_short_code: &str) -> String {
    stored_short_code.to_uppercase()
}

/// Validates the `session_id`/`code` input for authorize.
pub fn normalize_code_or_session(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request("Missing 'session_id' or 'code'."));
    }
    Ok(trimmed.to_string())
}

/// Validates the `session_id` input for check.
pub fn normalize_session_lookup(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request("Missing 'session_id'."));
    }
    Ok(trimmed.to_string())
}

/// Normalizes an optional device name, falling back to the default.
pub fn normalize_device_name(name: Option<&str>) -> String {
    name.map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(DEFAULT_DEVICE_NAME)
        .to_string()
}
