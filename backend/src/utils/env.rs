#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use worker::Env;

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use crate::common::{
    ENV_ENVIRONMENT, ENV_FRONTEND_ORIGINS, SECRET_GOOGLE_CLIENT_ID,
};
use crate::models::error::{AppError, AppResult};

/// Pure helper: returns true if the raw environment string is "production".
pub fn parse_is_production(val: Option<&str>) -> bool {
    val.map(|v| v.trim() == "production").unwrap_or(false)
}

/// Pure helper: returns the environment name, defaulting to "dev" if unset or blank.
pub fn parse_environment(val: Option<&str>) -> String {
    val.map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "dev".to_string())
}

/// Pure helper: returns None if unset or only whitespace; Some(trimmed) otherwise.
pub fn parse_frontend_origins_value(val: Option<&str>) -> Option<String> {
    val.map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// Pure helper: validates that GOOGLE_CLIENT_ID is present and non-empty.
pub fn validate_google_client_id(secret: Option<&str>) -> AppResult<String> {
    let secret = secret
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::service_unavailable("Google login is not configured on the server."))?;
    Ok(secret.to_string())
}

/// Returns true if the ENVIRONMENT variable is set to "production".
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub fn is_production(env: &Env) -> bool {
    let var = env.var(ENV_ENVIRONMENT).map(|v| v.to_string()).ok();
    parse_is_production(var.as_deref())
}

/// Returns the ENVIRONMENT variable, defaulting to "dev" if unset.
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub fn get_environment(env: &Env) -> String {
    let var = env.var(ENV_ENVIRONMENT).map(|v| v.to_string()).ok();
    parse_environment(var.as_deref())
}

/// Returns the optional FRONTEND_ORIGINS variable string.
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub fn get_frontend_origins(env: &Env) -> Option<String> {
    let var = env.var(ENV_FRONTEND_ORIGINS).map(|v| v.to_string()).ok();
    parse_frontend_origins_value(var.as_deref())
}

/// Retrieves the GOOGLE_CLIENT_ID secret, returning 503 if missing or empty.
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub fn get_google_client_id(env: &Env) -> AppResult<String> {
    let secret = env
        .secret(SECRET_GOOGLE_CLIENT_ID)
        .map(|s| s.to_string())
        .ok();
    validate_google_client_id(secret.as_deref())
}
