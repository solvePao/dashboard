use serde::{Deserialize, Serialize};
use super::auth::User;
use crate::common::DeviceLifecycleKind;

/// D1 pairing-code index row: a pointer only. Pairing state lives in
/// the `PairingObject` Durable Object, never in this table.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PairingCode {
    pub session_id: String,
    pub short_code: String,
    pub expires_at: String,
}

/// Worker -> Durable Object: start (or refresh) a pending pairing.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PairingInitRequest {
    pub expires_at: String,
}

/// Worker -> Durable Object: commit a minted token for a pending pairing.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PairingCommitRequest {
    pub token: String,
    pub user: User,
}

/// Durable Object pairing snapshot, shared by commit/status/push.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PairingState {
    pub status: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Worker -> hub object: one lifecycle change (authorize/revoke/disconnect).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HubEventRequest {
    pub kind: DeviceLifecycleKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// A lifecycle change as stored and broadcast by the hub object.
/// `revision` is assigned by the hub and strictly increases per user.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HubEvent {
    pub kind: DeviceLifecycleKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub revision: u64,
    /// Unix millis when the hub recorded the event.
    pub at: u64,
}

/// Hub storage record: one small value per user, written once per
/// lifecycle event and read only when a dashboard socket connects.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HubState {
    pub revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event: Option<HubEvent>,
}

/// Sent to a dashboard socket on connect so it can replay a missed
/// `last_event` or detect a larger revision gap and re-fetch.
#[derive(Debug, Clone, Serialize)]
pub struct HubSnapshot {
    pub kind: String,
    pub revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event: Option<HubEvent>,
}

