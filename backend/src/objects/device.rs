#[cfg(not(any(test, coverage)))]
use worker::*;

#[cfg(not(any(test, coverage)))]
use crate::models::device::{HubEvent, HubEventRequest, HubSnapshot, HubState};

#[cfg(not(any(test, coverage)))]
const HUB_STORAGE_KEY: &str = "hub";

/// One instance per active device token (`idFromName(device_token)`).
/// Manages live WebSocket push for real-time revocation when the web
/// dashboard deletes a device token.
#[cfg(not(any(test, coverage)))]
#[durable_object]
pub struct DeviceObject {
    state: State,
}

#[cfg(not(any(test, coverage)))]
#[durable_object]
pub struct DevicesHubObject {
    state: State,
}


/// Payload pushed to live device sockets on revocation. Pure so the
/// contract both sides depend on is unit-tested.
pub fn revoked_payload() -> serde_json::Value {
    serde_json::json!({
        "status": "revoked",
        "message": "Device access was revoked from the web dashboard."
    })
}

/// Payload pushed to live device sockets when the web dashboard saves
/// widgets. The socket stays open; the device re-fetches its widget set.
pub fn widgets_updated_payload() -> serde_json::Value {
    serde_json::json!({
        "status": "widgets_updated",
        "message": "Widget configuration was updated from the web dashboard."
    })
}

/// Payload pushed to live device sockets when an incoming walkie-talkie call/activation occurs.
pub fn walkie_activate_payload(group_id: &str) -> serde_json::Value {
    serde_json::json!({
        "status": "walkie_activate",
        "group_id": group_id,
        "message": "Incoming walkie-talkie call."
    })
}

#[cfg(not(any(test, coverage)))]
impl DeviceObject {
    fn broadcast_revoked(&self) {
        let msg = revoked_payload().to_string();

        for ws in self.state.get_websockets() {
            let _ = ws.send_with_str(&msg);
            let _ = ws.close(Some(1000), Some("device revoked"));
        }
    }

    fn broadcast_widgets_updated(&self) {
        let msg = widgets_updated_payload().to_string();

        for ws in self.state.get_websockets() {
            let _ = ws.send_with_str(&msg);
        }
    }

    async fn handle_revoke(&self) -> Result<Response> {
        self.broadcast_revoked();
        let _ = self.state.storage().delete_all().await;
        Response::ok("revoked")
    }

    async fn handle_widgets_updated(&self) -> Result<Response> {
        self.broadcast_widgets_updated();
        Response::ok("widgets_updated")
    }

    fn broadcast_walkie_activate(&self, group_id: &str) {
        let msg = walkie_activate_payload(group_id).to_string();

        for ws in self.state.get_websockets() {
            let _ = ws.send_with_str(&msg);
        }
    }

    async fn handle_walkie_activate(&self, mut req: Request) -> Result<Response> {
        let body: serde_json::Value = req.json().await.unwrap_or(serde_json::json!({}));
        let group_id = body
            .get("group_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        self.broadcast_walkie_activate(group_id);
        Response::ok("walkie_activated")
    }

    async fn handle_socket(&self) -> Result<Response> {
        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        Response::from_websocket(pair.client)
    }
}

#[cfg(not(any(test, coverage)))]
impl DurableObject for DeviceObject {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        let is_socket = req
            .headers()
            .get("upgrade")
            .ok()
            .flatten()
            .map(|v| v.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);

        match (req.method(), req.path().as_str()) {
            (Method::Post, "/revoke") => self.handle_revoke().await,
            (Method::Post, "/widgets") => self.handle_widgets_updated().await,
            (Method::Post, "/walkie-activate") => self.handle_walkie_activate(req).await,
            (Method::Get, _) if is_socket => self.handle_socket().await,
            _ => Response::error("Not found.", 404),
        }
    }

    async fn websocket_message(
        &self,
        _ws: WebSocket,
        _message: WebSocketIncomingMessage,
    ) -> Result<()> {
        Ok(())
    }

    async fn websocket_close(
        &self,
        _ws: WebSocket,
        _code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, _error: Error) -> Result<()> {
        Ok(())
    }
}

#[cfg(not(any(test, coverage)))]
impl DevicesHubObject {
    async fn load(&self) -> Result<HubState> {
        Ok(self
            .state
            .storage()
            .get::<HubState>(HUB_STORAGE_KEY)
            .await?
            .unwrap_or_default())
    }

    async fn handle_event(&self, mut req: Request) -> Result<Response> {
        let body: HubEventRequest = match req.json().await {
            Ok(b) => b,
            Err(_) => return Response::error("Invalid JSON body.", 400),
        };
        let mut hub = self.load().await?;
        hub.revision = hub.revision.saturating_add(1);
        let event = HubEvent {
            kind: body.kind,
            token: body.token,
            revision: hub.revision,
            at: Date::now().as_millis(),
        };
        hub.last_event = Some(event.clone());
        self.state.storage().put(HUB_STORAGE_KEY, &hub).await?;

        if let Ok(msg) = serde_json::to_string(&event) {
            for ws in self.state.get_websockets() {
                let _ = ws.send_with_str(&msg);
            }
        }
        Response::from_json(&event)
    }

    async fn handle_socket(&self) -> Result<Response> {
        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        let hub = self.load().await?;
        let snapshot = HubSnapshot {
            kind: "snapshot".to_string(),
            revision: hub.revision,
            last_event: hub.last_event,
        };
        if let Ok(msg) = serde_json::to_string(&snapshot) {
            let _ = pair.server.send_with_str(msg);
        }
        Response::from_websocket(pair.client)
    }
}

#[cfg(not(any(test, coverage)))]
impl DurableObject for DevicesHubObject {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        let is_socket = req
            .headers()
            .get("upgrade")
            .ok()
            .flatten()
            .map(|v| v.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);

        match (req.method(), req.path().as_str()) {
            (Method::Post, "/event") => self.handle_event(req).await,
            (Method::Get, _) if is_socket => self.handle_socket().await,
            _ => Response::error("Not found.", 404),
        }
    }

    async fn websocket_message(
        &self,
        _ws: WebSocket,
        _message: WebSocketIncomingMessage,
    ) -> Result<()> {
        Ok(())
    }

    async fn websocket_close(
        &self,
        _ws: WebSocket,
        _code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, _error: Error) -> Result<()> {
        Ok(())
    }
}

