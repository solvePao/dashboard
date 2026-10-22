pub mod admission;
pub mod api;
pub mod enrollment;
pub mod repository;
pub mod room;
pub mod sfu;

use crate::models::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use worker::*;

pub fn db_error(_: impl std::fmt::Display) -> AppError {
    console_error!("walkie database operation failed");
    AppError::internal("Could not update walkie-talkie. Please retry.")
}

pub fn random_id() -> AppResult<String> {
    crate::utils::generate_secure_token().map_err(AppError::internal)
}

pub fn now() -> u64 {
    Date::now().as_millis()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Member {
    pub device_id: String,
    pub user_id: i64,
    pub name: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub device_name: String,
    #[serde(default)]
    pub picture_url: Option<String>,
    #[serde(default)]
    pub online: bool,
    #[serde(default)]
    pub audio_active: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub owner_id: i64,
    #[serde(default)]
    pub members: Vec<Member>,
    #[serde(default)]
    #[serde(deserialize_with = "crate::db::helper::sqlite_bool")]
    pub joined: bool,
    #[serde(default)]
    #[serde(deserialize_with = "crate::db::helper::sqlite_bool")]
    pub active: bool,
}

pub async fn room_call(
    env: &Env,
    group: &str,
    path: &str,
    body: &serde_json::Value,
) -> AppResult<Response> {
    let stub = env
        .durable_object("WALKIE_GROUP")
        .and_then(|ns| ns.get_by_name(group))
        .map_err(|_| AppError::service_unavailable("Walkie-talkie coordinator unavailable."))?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(body.to_string().into()));
    let req = Request::new_with_init(&format!("https://walkie{path}"), &init).map_err(db_error)?;
    stub.fetch_with_request(req)
        .await
        .map_err(|_| AppError::service_unavailable("Walkie-talkie coordinator unavailable."))
}
