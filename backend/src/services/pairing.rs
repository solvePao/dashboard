#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use wasm_bindgen::JsValue;
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use worker::*;

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use super::store::{DeviceNotifier, HubNotifier, PairingClient};
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use crate::common::{DeviceLifecycleKind, PAIRING_BINDING_NAME};
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use crate::models::auth::User;
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use crate::models::device::{
    CheckSessionResponse, HubEventRequest, PairingCommitRequest, PairingInitRequest, PairingState,
};
use crate::models::error::{AppError, AppResult};

/// Production `PairingClient` over a Durable Object namespace.
/// One stub per session, addressed by the full `session_id`.
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub struct DoPairingClient {
    namespace: ObjectNamespace,
}

/// Production `DeviceNotifier` over a Durable Object namespace.
/// One stub per device token, addressed by the token itself.
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub struct DoDeviceNotifier {
    namespace: ObjectNamespace,
}

/// Production `HubNotifier` over a Durable Object namespace.
/// One stub per user, addressed by the internal user id.
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub struct DoHubNotifier {
    namespace: ObjectNamespace,
}

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
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
        let _ = headers.set("x-session-id", session_id);
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

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
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

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
impl DoDeviceNotifier {
    pub fn from_env(env: &Env) -> AppResult<Self> {
        let namespace = env.durable_object(crate::common::GATEWAY_BINDING_NAME).map_err(|_| {
            AppError::service_unavailable(format!(
                "Durable Object binding '{}' is not configured.",
                crate::common::GATEWAY_BINDING_NAME
            ))
        })?;
        Ok(Self { namespace })
    }

    async fn deliver(&self, device_token: &str, payload: serde_json::Value) {
        let Ok(stub) = self.namespace.get_by_name(&crate::common::gateway_device_name(device_token)) else { return };
        let body = serde_json::json!({"channel": "device", "payload": payload});
        let headers = Headers::new();
        let _ = headers.set("content-type", "application/json");
        let mut init = RequestInit::new();
        init.method = Method::Post;
        init.headers = headers;
        init.body = Some(body.to_string().into());
        let Ok(req) = Request::new_with_init("https://gateway/deliver", &init) else { return };
        let _ = stub.fetch_with_request(req).await;
    }
}

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
impl DeviceNotifier for DoDeviceNotifier {
    async fn notify_revoked(&self, device_token: &str) {
        self.deliver(device_token, crate::objects::revoked_payload()).await;
    }

    async fn notify_widgets_updated(&self, device_token: &str) {
        self.deliver(device_token, crate::objects::widgets_updated_payload()).await;
    }

    async fn notify_walkie_activate(&self, device_token: &str, group_id: &str) {
        self.deliver(device_token, crate::objects::walkie_activate_payload(group_id)).await;
    }
}

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
impl DoHubNotifier {
    pub fn from_env(env: &Env) -> AppResult<Self> {
        let namespace = env.durable_object(crate::common::GATEWAY_BINDING_NAME).map_err(|_| {
            AppError::service_unavailable(format!(
                "Durable Object binding '{}' is not configured.",
                crate::common::GATEWAY_BINDING_NAME
            ))
        })?;
        Ok(Self { namespace })
    }
}

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
impl HubNotifier for DoHubNotifier {
    async fn notify_hub(&self, user_id: i64, kind: DeviceLifecycleKind, device_token: Option<&str>) {
        // Best-effort push; errors are intentionally swallowed.
        let Ok(stub) = self.namespace.get_by_name(&crate::common::gateway_user_name(user_id)) else { return };
        let Ok(json) = serde_json::to_string(&HubEventRequest {
            kind,
            token: device_token.map(str::to_string),
        }) else {
            return;
        };
        let headers = Headers::new();
        if headers.set("content-type", "application/json").is_err() {
            return;
        }
        let mut init = RequestInit::new();
        init.method = Method::Post;
        init.headers = headers;
        init.body = Some(JsValue::from_str(&json));
        let Ok(req) = Request::new_with_init("https://gateway/hub-event", &init) else { return };
        let _ = stub.fetch_with_request(req).await;
    }
}
