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

pub const USER_COLUMNS: &str =
    "id, google_sub, email, name, picture_url, created_at, updated_at, last_login_at";

pub const SQL_NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
pub const SESSION_EXPIRY_SQL: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+15 minutes')";
pub const TOKEN_EXPIRY_SQL: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+180 days')";

pub const SESSION_ID_HEX_LEN: usize = 128;
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


