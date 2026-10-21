use crate::common::{DEFAULT_DEVICE_NAME, SHORT_CODE_LEN};
use crate::models::error::{AppError, AppResult};
use crate::utils::is_valid_session_id_hex;

/// Validates a raw session id: trims and enforces 32-hex shape.
/// Mirrors `DeviceService::create_session` validation exactly.
pub fn validate_session_id(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if !is_valid_session_id_hex(trimmed) {
        return Err(AppError::bad_request(
            "Invalid session_id. Must be a 32-character hex string.",
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

/// Longest accepted widget key. Keys are client-registry names
/// (`weather`, later `stocks`, `gallery`, ...).
pub const MAX_WIDGET_KEY_LEN: usize = 32;

/// Upper bound on widgets stored per device; keeps a PUT body small.
pub const MAX_WIDGETS_PER_DEVICE: usize = 16;

/// Validates one widget key: `1..=32` chars of lowercase letters,
/// digits, or underscores. The backend is intentionally key-agnostic —
/// the widget registry lives on the frontends — so unknown-but-shaped
/// keys are accepted and simply render nothing until a client knows them.
pub fn normalize_widget_key(raw: Option<&str>) -> AppResult<String> {
    let trimmed = raw.map(str::trim).unwrap_or("");
    if trimmed.is_empty() {
        return Err(AppError::bad_request("Missing 'widget_key'."));
    }
    let shaped = !trimmed.is_empty()
        && trimmed.len() <= MAX_WIDGET_KEY_LEN
        && trimmed
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !shaped {
        return Err(AppError::bad_request(
            "Invalid 'widget_key'. Use 1-32 lowercase letters, digits, or underscores.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates one widget config: must be a JSON object when present,
/// defaulting to `{}` so callers can omit it.
pub fn normalize_widget_config(raw: Option<&serde_json::Value>) -> AppResult<serde_json::Value> {
    match raw {
        None => Ok(serde_json::json!({})),
        Some(v) if v.is_object() => Ok(v.clone()),
        Some(_) => Err(AppError::bad_request("Invalid 'config'. Must be a JSON object.")),
    }
}

/// Validates a walkie-talkie device code: exactly 6 ASCII digits.
/// Mirrors `walkie::admission` code checks.
pub fn validate_device_code(code: &str) -> AppResult<()> {
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::bad_request("Enter a six-digit device code."));
    }
    Ok(())
}

/// Normalizes a walkie-talkie group name: trims and enforces 1–60 chars.
/// Mirrors `walkie::api` group-name checks.
pub fn normalize_group_name(raw: Option<&str>) -> AppResult<String> {
    let name = raw.map(str::trim).unwrap_or("");
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AppError::bad_request("Use a group name of 1–60 characters."));
    }
    Ok(name.to_string())
}

/// Validates a full PUT widgets body into normalized widgets. Missing
/// body or duplicate keys are 400s; an empty list is valid and clears
/// every widget (the web "None" choice).
pub fn normalize_widgets(
    items: Option<&[crate::models::device::WidgetSaveItem]>,
) -> AppResult<Vec<crate::models::device::DeviceWidget>> {
    let items = items
        .ok_or_else(|| AppError::bad_request("Missing 'widgets'. Expected a list of widgets."))?;
    if items.len() > MAX_WIDGETS_PER_DEVICE {
        return Err(AppError::bad_request("Too many widgets."));
    }
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let widget = crate::models::device::DeviceWidget {
            widget_key: normalize_widget_key(item.widget_key.as_deref())?,
            enabled: item.enabled.unwrap_or(true),
            config: normalize_widget_config(item.config.as_ref())?,
        };
        if out.iter().any(|w: &crate::models::device::DeviceWidget| w.widget_key == widget.widget_key) {
            return Err(AppError::bad_request("Duplicate 'widget_key'."));
        }
        out.push(widget);
    }
    Ok(out)
}
