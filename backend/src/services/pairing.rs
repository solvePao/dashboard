use wasm_bindgen::JsValue;
use worker::*;

use super::store::{DeviceNotifier, PairingClient};
use crate::common::{DEVICE_BINDING_NAME, PAIRING_BINDING_NAME};
use crate::models::auth::User;
use crate::models::device::{
    CheckSessionResponse, PairingCommitRequest, PairingInitRequest, PairingState,
};
use crate::models::error::{AppError, AppResult};

/// Production `PairingClient` over a Durable Object namespace.
/// One stub per session, addressed by the full `session_id`.
pub struct DoPairingClient {
    namespace: ObjectNamespace,
}

impl DoPairingClient {
    pub fn from_env(env: &Env) -> AppResult<Self> {
        let namespace = env
            .durable_object(PAIRING_BINDING_NAME)
            .map_err(|_| {
                AppError::service_unavailable(format!(
                    "Durable Object binding '{PAIRING_BINDING_NAME}' is not configured."
                ))
            })?;
        Ok(Self { namespace })
    }

    /// Raw namespace access for the WebSocket proxy route.
    pub fn namespace(&self) -> &ObjectNamespace {
        &self.namespace
    }

    fn stub(&self, session_id: &str) -> AppResult<Stub> {
        self.namespace
            .get_by_name(session_id)
            .map_err(|_| AppError::bad_gateway("Failed to reach pairing session."))
    }

    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        session_id: &str,
        path: &str,
        body: &B,
    ) -> AppResult<(u16, Option<T>)> {
        let json = serde_json::to_string(body)
            .map_err(|_| AppError::internal("Failed to encode pairing request."))?;
        let headers = Headers::new();
        headers
            .set("content-type", "application/json")
            .map_err(|_| AppError::internal("Failed to build pairing request."))?;
        let mut init = RequestInit::new();
        init.method = Method::Post;
        init.headers = headers;
        init.body = Some(JsValue::from_str(&json));
        let req = Request::new_with_init(&format!("https://pairing{path}"), &init)
            .map_err(|_| AppError::internal("Failed to build pairing request."))?;
        let mut res = self
            .stub(session_id)?
            .fetch_with_request(req)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to reach pairing session."))?;
        let status = res.status_code();
        if status == 404 || status == 409 {
            return Ok((status, None));
        }
        if !(200..300).contains(&status) {
            return Err(AppError::bad_gateway("Pairing session error."));
        }
        let parsed = res
            .json::<T>()
            .await
            .map_err(|_| AppError::bad_gateway("Pairing session error."))?;
        Ok((status, Some(parsed)))
    }
}

impl PairingClient for DoPairingClient {
    async fn client_init(&self, session_id: &str, expires_at: &str) -> AppResult<PairingState> {
        let (_, parsed) = self
            .post_json(
                session_id,
                "/init",
                &PairingInitRequest {
                    expires_at: expires_at.to_string(),
                },
            )
            .await?;
        parsed.ok_or_else(|| AppError::bad_gateway("Pairing session error."))
    }

    async fn client_status(&self, session_id: &str) -> AppResult<Option<CheckSessionResponse>> {
        let req = Request::new("https://pairing/status", Method::Get)
            .map_err(|_| AppError::internal("Failed to build pairing request."))?;
        let mut res = self
            .stub(session_id)?
            .fetch_with_request(req)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to reach pairing session."))?;
        if res.status_code() == 404 {
            return Ok(None);
        }
        if !(200..300).contains(&res.status_code()) {
            return Err(AppError::bad_gateway("Pairing session error."));
        }
        let parsed = res
            .json::<CheckSessionResponse>()
            .await
            .map_err(|_| AppError::bad_gateway("Pairing session error."))?;
        Ok(Some(parsed))
    }

    async fn client_commit(
        &self,
        session_id: &str,
        device_token: &str,
        user: &User,
    ) -> AppResult<bool> {
        let (status, _) = self
            .post_json::<_, PairingState>(
                session_id,
                "/commit",
                &PairingCommitRequest {
                    token: device_token.to_string(),
                    user: user.clone(),
                },
            )
            .await?;
        map_commit_status(status)
    }
}

/// Pure mapping of the object's commit response. `false` means the
/// object refused because the pairing already finished (409 race).
pub fn map_commit_status(status: u16) -> AppResult<bool> {
    match status {
        409 => Ok(false),
        404 => Err(AppError::not_found("Pending session not found or expired.")),
        _ => Ok(true),
    }
}

/// Production `DeviceNotifier` over a Durable Object namespace.
/// One stub per device token, addressed by the token itself.
pub struct DoDeviceNotifier {
    namespace: ObjectNamespace,
}

impl DoDeviceNotifier {
    pub fn from_env(env: &Env) -> AppResult<Self> {
        let namespace = env.durable_object(DEVICE_BINDING_NAME).map_err(|_| {
            AppError::service_unavailable(format!(
                "Durable Object binding '{DEVICE_BINDING_NAME}' is not configured."
            ))
        })?;
        Ok(Self { namespace })
    }
}

impl DeviceNotifier for DoDeviceNotifier {
    async fn notify_revoked(&self, device_token: &str) {
        // Best-effort push; errors are intentionally swallowed.
        let Ok(stub) = self.namespace.get_by_name(device_token) else { return };
        let mut init = RequestInit::new();
        init.method = Method::Post;
        let Ok(req) = Request::new_with_init("https://device/revoke", &init) else { return };
        let _ = stub.fetch_with_request(req).await;
    }
}
