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

