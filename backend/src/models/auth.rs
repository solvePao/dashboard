use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub google_sub: String,
    pub email: String,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GoogleProfile {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenInfo {
    pub aud: Option<String>,
    pub iss: Option<String>,
    pub sub: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub exp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleAuthRequest {
    pub credential: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user: User,
}

/// Server-minted web session token claims (HS256).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Google subject id — the stable user identity.
    pub sub: String,
    /// Internal user row id, cross-checked against the DB on every request.
    pub uid: i64,
    /// Expiration and issuance, in unix seconds.
    pub exp: u64,
    pub iat: u64,
    /// Optional not-before (unix seconds). Minted tokens omit it; foreign
    /// tokens carrying a future `nbf` are rejected.
    #[serde(default)]
    pub nbf: Option<u64>,
}

/// Returned by `POST /api/auth/google`: the user plus the JWT the
/// frontend must use as `Bearer` on every other route.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: User,
    pub token: String,
    /// Unix millis when `token` expires (mirrors the JWT `exp`).
    pub expires_at: u64,
}

