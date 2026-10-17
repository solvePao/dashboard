pub const DEFAULT_ORIGINS: &[&str] = &[
    "http://localhost:5173",
    "http://127.0.0.1:5173",
    "http://localhost:5174",
    "http://127.0.0.1:5174",
];

pub const ALLOWED_METHODS: &str = "GET, POST, DELETE, OPTIONS";
pub const ALLOWED_HEADERS: &str = "Content-Type, Authorization";
pub const CORS_MAX_AGE: &str = "86400";

pub const DB_BINDING_NAME: &str = "dashboard";
pub const PAIRING_BINDING_NAME: &str = "PAIRING_OBJECT";
pub const DEVICE_BINDING_NAME: &str = "DEVICE_OBJECT";
pub const HUB_BINDING_NAME: &str = "DEVICES_HUB";

/// Cookie carrying the web session JWT for the hub WebSocket only.
/// REST routes keep using the `Authorization: Bearer` header.
pub const SESSION_COOKIE_NAME: &str = "dashboard_session";

pub const USER_COLUMNS: &str =
    "id, google_sub, email, name, picture_url, created_at, updated_at, last_login_at";

pub const SQL_NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
pub const SESSION_EXPIRY_SQL: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+2 minutes')";
/// Pairing session lifetime in seconds; mirrors `SESSION_EXPIRY_SQL`.
/// The Durable Object alarm and the D1 code index share this clock.
pub const SESSION_TTL_SECS: u64 = 120;
pub const TOKEN_EXPIRY_SQL: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+180 days')";

pub const SESSION_ID_HEX_LEN: usize = 32;
pub const SHORT_CODE_LEN: usize = 6;
pub const DEFAULT_DEVICE_NAME: &str = "Mobile Device";

pub const JWT_SECRET_NAME: &str = "JWT_SECRET";
/// Web session JWT lifetime: 60 minutes, in seconds and millis.
pub const JWT_EXPIRY_SECS: u64 = 3600;
pub const JWT_EXPIRY_MILLIS: u64 = JWT_EXPIRY_SECS * 1000;

/// Minimum accepted JWT secret length. Rejects empty/missing secrets (fail
/// closed with 503) and trivially short ones.
pub const MIN_JWT_SECRET_LEN: usize = 32;

/// Leeway for `iat` future-skew checks, in seconds.
pub const JWT_CLOCK_SKEW_SECS: u64 = 60;

/// Delay in seconds before answering failed device authorization attempts.
pub const FAILED_AUTHORIZE_DELAY_SECS: u64 = 5;

pub const ENV_ENVIRONMENT: &str = "ENVIRONMENT";
pub const ENV_FRONTEND_ORIGINS: &str = "FRONTEND_ORIGINS";
pub const SECRET_GOOGLE_CLIENT_ID: &str = "GOOGLE_CLIENT_ID";

use serde::{Deserialize, Serialize};

/// Lifecycle change events broadcast to connected dashboards and device sockets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceLifecycleKind {
    Authorized,
    Revoked,
    Disconnected,
}

impl DeviceLifecycleKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Authorized => "authorized",
            Self::Revoked => "revoked",
            Self::Disconnected => "disconnected",
        }
    }
}

impl std::fmt::Display for DeviceLifecycleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}


