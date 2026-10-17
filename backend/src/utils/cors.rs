use worker::*;

use crate::common::{
    ALLOWED_HEADERS, ALLOWED_METHODS, CORS_MAX_AGE, DEFAULT_ORIGINS,
};

/// Pure parsing shared by production and tests.
pub fn parse_allowed_origins(configured: &str) -> Vec<String> {
    let parsed: Vec<String> = configured
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if parsed.is_empty() {
        DEFAULT_ORIGINS.iter().map(|s| s.to_string()).collect()
    } else {
        parsed
    }
}

/// Pure origin check shared by `apply_cors` / `cors_preflight`.
pub fn is_origin_allowed(origin: Option<&str>, allowed: &[String]) -> bool {
    match origin {
        Some(o) => allowed.iter().any(|a| a == o),
        None => false,
    }
}

/// Origins allowed to call the API. Configured with the comma-separated
/// `FRONTEND_ORIGINS` var; falls back to the local Vite dev servers.
pub fn allowed_origins(env: &Env) -> Vec<String> {
    let configured = super::env::get_frontend_origins(env).unwrap_or_default();
    parse_allowed_origins(&configured)
}

pub fn request_origin(req: &Request) -> Option<String> {
    req.headers().get("Origin").unwrap_or(None)
}

/// True for a WebSocket upgrade response. Upgrades bypass CORS
/// mutation: the handshake is not subject to CORS and its headers may
/// be immutable (mutating them throws at runtime).
pub fn is_websocket_upgrade(status_code: u16) -> bool {
    status_code == 101
}

pub fn apply_cors(origin: Option<&str>, allowed: &[String], mut res: Response) -> Result<Response> {
    if is_origin_allowed(origin, allowed) {
        if let Some(origin) = origin {
            let headers = res.headers_mut();
            headers.set("Access-Control-Allow-Origin", origin)?;
            // Credentialed requests (the login cookie) require the exact
            // origin above plus this flag; a wildcard origin is never used.
            headers.set("Access-Control-Allow-Credentials", "true")?;
            headers.set("Vary", "Origin")?;
        }
    }
    Ok(res)
}

pub fn cors_preflight(origin: Option<&str>, allowed: &[String]) -> Result<Response> {
    let mut res = Response::empty()?.with_status(204);
    if is_origin_allowed(origin, allowed) {
        if let Some(origin) = origin {
            let headers = res.headers_mut();
            headers.set("Access-Control-Allow-Origin", origin)?;
            headers.set("Access-Control-Allow-Credentials", "true")?;
            headers.set("Vary", "Origin")?;
            headers.set("Access-Control-Allow-Methods", ALLOWED_METHODS)?;
            headers.set("Access-Control-Allow-Headers", ALLOWED_HEADERS)?;
            headers.set("Access-Control-Max-Age", CORS_MAX_AGE)?;
        }
    }
    Ok(res)
}
