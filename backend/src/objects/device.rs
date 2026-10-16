use worker::*;

/// One instance per active device token (`idFromName(device_token)`).
/// Manages live WebSocket push for real-time revocation when the web
/// dashboard deletes a device token.
#[durable_object]
pub struct DeviceObject {
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

impl DeviceObject {
    fn broadcast_revoked(&self) {
        let msg = revoked_payload().to_string();

        for ws in self.state.get_websockets() {
            let _ = ws.send_with_str(&msg);
            let _ = ws.close(Some(1000), Some("device revoked"));
        }
    }

    async fn handle_revoke(&self) -> Result<Response> {
        self.broadcast_revoked();
        let _ = self.state.storage().delete_all().await;
        Response::ok("revoked")
    }

    async fn handle_socket(&self) -> Result<Response> {
        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        Response::from_websocket(pair.client)
    }
}

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

