use serde::{Deserialize, Serialize};
use super::auth::User;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceSession {
    pub session_id: String,
    pub short_code: String,
    pub status: String,
    pub user_id: Option<i64>,
    pub device_token: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceToken {
    pub token: String,
    pub user_id: i64,
    pub device_name: String,
    pub created_at: String,
    pub expires_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub short_code: String,
    pub qr_svg: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeDeviceRequest {
    pub session_id: Option<String>,
    pub code: Option<String>,
    pub device_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthorizeDeviceResponse {
    pub status: String,
    pub session_id: String,
    pub device_name: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckSessionRequest {
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckSessionResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
}

#[derive(Debug, Serialize)]
pub struct DevicesResponse {
    pub devices: Vec<DeviceToken>,
}

#[derive(Debug, Serialize)]
pub struct RevokeResponse {
    pub revoked: bool,
}

