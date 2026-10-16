use std::time::Duration;

use worker::*;

use crate::common::SESSION_TTL_SECS;
use crate::models::device::{
    CheckSessionResponse, PairingCommitRequest, PairingInitRequest, PairingState,
};

use super::state::{self, AlarmOutcome, CommitOutcome};

const STORAGE_KEY: &str = "pairing";

/// One instance per pairing session (`idFromName(session_id)`).
/// Sole referee for pairing state: pending -> authorized | expired.
/// D1 holds only the code index and long-lived device tokens.
#[durable_object]
pub struct PairingObject {
    state: State,
}

impl PairingObject {
    async fn load(&self) -> Result<Option<PairingState>> {
        self.state.storage().get(STORAGE_KEY).await
    }

    async fn save(&self, pairing: &PairingState) -> Result<()> {
        self.state.storage().put(STORAGE_KEY, pairing).await
    }

    fn broadcast(&self, payload: &CheckSessionResponse) {
        let Ok(msg) = serde_json::to_string(payload) else {
            return;
        };
        for ws in self.state.get_websockets() {
            let _ = ws.send_with_str(&msg);
        }
    }

    fn close_sockets(&self) {
        for ws in self.state.get_websockets() {
            let _ = ws.close(Some(1000), Some("pairing finished"));
        }
    }

    async fn handle_init(&self, mut req: Request) -> Result<Response> {
        let body: PairingInitRequest = match req.json().await {
            Ok(b) => b,
            Err(_) => return Response::error("Invalid JSON body.", 400),
        };
        let pairing = state::init_pairing(self.load().await?.as_ref(), &body.expires_at);
        self.save(&pairing).await?;
        self.state
            .storage()
            .set_alarm(Duration::from_secs(SESSION_TTL_SECS))
            .await?;
        Response::from_json(&pairing)
    }

    async fn handle_commit(&self, mut req: Request) -> Result<Response> {
        let body: PairingCommitRequest = match req.json().await {
            Ok(b) => b,
            Err(_) => return Response::error("Invalid JSON body.", 400),
        };
        match state::commit_pairing(self.load().await?.as_ref(), &body.token, &body.user) {
            CommitOutcome::AlreadyAuthorized => {
                Response::error("Session was already authorized.", 409)
            }
            CommitOutcome::MissingOrExpired => {
                Response::error("Pending session not found or expired.", 404)
            }
            CommitOutcome::Committed(pairing) => {
                self.save(&pairing).await?;
                self.broadcast(&state::as_check(&pairing));
                self.close_sockets();
                Response::from_json(&pairing)
            }
        }
    }

    async fn handle_status(&self) -> Result<Response> {
        match self.load().await? {
            Some(pairing) => Response::from_json(&state::as_check(&pairing)),
            None => Response::error("Session not found.", 404),
        }
    }

    async fn handle_socket(&self) -> Result<Response> {
        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        // Replay a finished result to a (re)connecting phone, then close.
        if let Some(pairing) = self.load().await? {
            if pairing.status != "pending" {
                let msg =
                    serde_json::to_string(&state::as_check(&pairing)).unwrap_or_default();
                let _ = pair.server.send_with_str(msg);
                let _ = pair.server.close(Some(1000), Some("pairing finished"));
            }
        }
        Response::from_websocket(pair.client)
    }
}

impl DurableObject for PairingObject {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        // The Worker proxies the public `/api/auth/device/ws/:id` path
        // verbatim, so sockets are detected by header, not path.
        let is_socket = req
            .headers()
            .get("upgrade")
            .ok()
            .flatten()
            .map(|v| v.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);
        match (req.method(), req.path().as_str()) {
            (Method::Post, "/init") => self.handle_init(req).await,
            (Method::Post, "/commit") => self.handle_commit(req).await,
            (Method::Get, "/status") => self.handle_status().await,
            (Method::Get, _) if is_socket => self.handle_socket().await,
            _ => Response::error("Not found.", 404),
        }
    }

    async fn alarm(&self) -> Result<Response> {
        match state::expire_pairing(self.load().await?.as_ref()) {
            None => {}
            Some(AlarmOutcome::Expired(expired)) => {
                self.save(&expired).await?;
                self.broadcast(&state::as_check(&expired));
                self.close_sockets();
            }
            // The result was delivered (or is replayable no longer);
            // wipe the token copy once the window closes.
            Some(AlarmOutcome::Wipe) => {
                self.state.storage().delete_all().await?;
            }
        }
        Response::ok("")
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
