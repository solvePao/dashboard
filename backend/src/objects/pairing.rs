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
    env: Env,
}

impl PairingObject {
    async fn load(&self) -> Result<Option<PairingState>> {
        self.state.storage().get(STORAGE_KEY).await
    }

    async fn save(&self, pairing: &PairingState) -> Result<()> {
        self.state.storage().put(STORAGE_KEY, pairing).await
    }

    async fn broadcast(&self, payload: &CheckSessionResponse) {
        // Delivered on the client's single gateway socket, channel `pairing`.
        if let Ok(value) = serde_json::to_value(payload) {
            if let Ok(Some(name)) = self.state.storage().get::<String>("gateway_name").await {
                super::gateway::deliver_to_gateway(&self.env, &name, "pairing", None, &value).await;
            }
        }
    }

    async fn remember_gateway(&self, req: &Request) {
        if let Ok(Some(session)) = req.headers().get("x-session-id") {
            let _ = self.state.storage().put("gateway_name", crate::common::gateway_pairing_name(&session)).await;
        }
    }

    async fn handle_init(&self, mut req: Request) -> Result<Response> {
        self.remember_gateway(&req).await;
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
        self.remember_gateway(&req).await;
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
                self.broadcast(&state::as_check(&pairing)).await;
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
}

impl DurableObject for PairingObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        match (req.method(), req.path().as_str()) {
            (Method::Post, "/init") => self.handle_init(req).await,
            (Method::Post, "/commit") => self.handle_commit(req).await,
            (Method::Get, "/status") => self.handle_status().await,
            _ => Response::error("Not found.", 404),
        }
    }

    async fn alarm(&self) -> Result<Response> {
        match state::expire_pairing(self.load().await?.as_ref()) {
            None => {}
            Some(AlarmOutcome::Expired(expired)) => {
                self.save(&expired).await?;
                self.broadcast(&state::as_check(&expired)).await;
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
