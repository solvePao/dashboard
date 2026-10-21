mod media;

use super::{repository::WalkieRepository, sfu::SfuService, *};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Clone, Default, Serialize, Deserialize)]
struct Room {
    group_id: String,
    clients: BTreeMap<String, Client>,
    speaker: Option<Speaker>,
    cleanup: Vec<MediaSession>,
    #[serde(default)]
    cleanup_in_flight: Option<MediaSession>,
    #[serde(default)]
    cleanup_started_at: u64,
    #[serde(default)]
    tickets: BTreeMap<String, Ticket>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Client {
    #[serde(default)]
    device_id: String,
    #[serde(default)]
    observe: bool,
    user_id: i64,
    token: String,
    name: String,
    #[serde(default)]
    first_name: String,
    #[serde(default)]
    picture_url: Option<String>,
    #[serde(default)]
    device_name: String,
    last_seen: u64,
    published: bool,
    sessions: Vec<MediaSession>,
}
#[derive(Clone, Serialize, Deserialize)]
struct MediaSession {
    id: String,
    #[serde(default)]
    pending: bool,
    mids: Vec<String>,
    target: Option<String>,
    #[serde(default)]
    attempts: u8,
}
#[derive(Clone, Serialize, Deserialize)]
struct Speaker {
    connection_id: String,
    user_id: i64,
    lease_id: String,
    expires_at: u64,
}
#[derive(Clone, Serialize, Deserialize)]
struct Ticket {
    #[serde(default)]
    observe: bool,
    token: String,
    group_id: String,
    expires_at: u64,
}
#[derive(Serialize, Deserialize)]
struct Attachment {
    connection_id: String,
}

#[durable_object]
#[derive(Clone)]
pub struct WalkieGroupObject {
    state: std::rc::Rc<State>,
    env: Env,
}

impl WalkieGroupObject {
    async fn load(&self) -> Result<Room> {
        Ok(self.state.storage().get("room").await?.unwrap_or_default())
    }

    async fn save(&self, room: &Room) -> Result<()> {
        self.state.storage().put("room", room).await?;
        if !room.clients.is_empty()
            || !room.cleanup.is_empty()
            || room.cleanup_in_flight.is_some()
            || !room.tickets.is_empty()
        {
            let delay = room
                .speaker
                .as_ref()
                .map(|s| s.expires_at.saturating_sub(now()).max(1))
                .unwrap_or(
                    if room.cleanup.is_empty() || room.cleanup_in_flight.is_some() {
                        15_000
                    } else {
                        1000
                    },
                );
            self.state
                .storage()
                .set_alarm(Duration::from_millis(delay.min(15_000)))
                .await?;
        } else {
            let _ = self.state.storage().delete_alarm().await;
        }
        Ok(())
    }

    fn send(&self, id: &str, value: &Value) {
        for ws in self.state.get_websockets() {
            if ws
                .deserialize_attachment::<Attachment>()
                .ok()
                .flatten()
                .is_some_and(|a| a.connection_id == id)
            {
                let _ = ws.send_with_str(value.to_string());
            }
        }
    }

    async fn broadcast(&self, room: &Room) -> Result<()> {
        let db = self.env.d1("dashboard")?;
        let repo = WalkieRepository(&db);
        let mut members = repo.members(&room.group_id).await.unwrap_or_default();
        for member in &mut members {
            member.online = room
                .clients
                .values()
                .any(|c| c.device_id == member.device_id);
            member.audio_active = room
                .clients
                .values()
                .any(|c| c.device_id == member.device_id && c.published);
        }
        let name = if let Some(c) = room.clients.values().next() {
            repo.member(&room.group_id, &c.token)
                .await
                .ok()
                .map(|g| g.name)
        } else {
            None
        };
        let value = json!({"type":"snapshot","group_name":name,"members":members,
            "clients":room.clients.iter().filter(|(_,c)|!c.observe).map(|(id,c)| json!({"connection_id":id,"device_id":c.device_id,"user_id":c.user_id,"name":c.first_name,"first_name":c.first_name,"picture_url":c.picture_url,"device_name":c.device_name,"published":c.published})).collect::<Vec<_>>(),
            "speaker":room.speaker.as_ref().map(|s| json!({"connection_id":s.connection_id,"device_id":room.clients.get(&s.connection_id).map(|c|&c.device_id),"user_id":s.user_id,"expires_at":s.expires_at}))});
        for ws in self.state.get_websockets() {
            let _ = ws.send_with_str(value.to_string());
        }
        Ok(())
    }

    fn remove(&self, room: &mut Room, id: &str, reason: &str) {
        if let Some(client) = room.clients.remove(id) {
            room.cleanup.extend(client.sessions);
        }
        for c in room.clients.values_mut() {
            let mut kept = Vec::new();
            for session in std::mem::take(&mut c.sessions) {
                if session.target.as_deref() == Some(id) {
                    room.cleanup.push(session);
                } else {
                    kept.push(session);
                }
            }
            c.sessions = kept;
        }
        if room.speaker.as_ref().is_some_and(|s| s.connection_id == id) {
            room.speaker = None;
        }
        self.send(id, &json!({"type":"ended","reason":reason}));
        for ws in self.state.get_websockets() {
            if ws
                .deserialize_attachment::<Attachment>()
                .ok()
                .flatten()
                .is_some_and(|a| a.connection_id == id)
            {
                let _ = ws.close(Some(1000), Some("walkie session ended"));
            }
        }
    }

    async fn prune_state(&self, room: &mut Room) -> Result<()> {
        room.tickets.retain(|_, t| t.expires_at > now());
        let db = self.env.d1("dashboard")?;
        let repo = WalkieRepository(&db);
        #[derive(Deserialize)]
        struct Authorized {
            token: String,
            user_id: i64,
            active: i64,
        }
        // Validate the room in one database read rather than one round trip per check per socket.
        let authorized = repo.rows::<Authorized>(
            "SELECT t.token,t.user_id,EXISTS(SELECT 1 FROM walkie_assignments a WHERE a.token=t.token AND a.group_id=m.group_id AND json_extract(w.config_json,'$.group_id')=m.group_id) AS active FROM device_tokens t JOIN walkie_device_members m ON m.token=t.token JOIN device_widgets w ON w.token=t.token AND w.widget_key='walkie_talkie' AND w.enabled=1 WHERE m.group_id=? AND t.expires_at>strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            &[D1Type::Text(&room.group_id)],
        ).await.map_err(|error| Error::RustError(error.message))?;
        let removed: Vec<String> = room
            .clients
            .iter()
            .filter(|(_, client)| {
                !authorized.iter().any(|entry| {
                    entry.token == client.token
                        && entry.user_id == client.user_id
                        && (client.observe || entry.active != 0)
                }) || client.device_id.is_empty()
                    || client.last_seen + 45_000 < now()
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in removed {
            self.remove(
                room,
                &id,
                "Access changed or connection timed out. Enable audio again.",
            );
        }
        if room.speaker.as_ref().is_some_and(|s| s.expires_at <= now()) {
            console_log!("walkie speaking lease expired");
            room.speaker = None;
        }
        Ok(())
    }

    async fn prune(&self, room: &mut Room) -> Result<()> {
        self.prune_state(room).await?;
        self.save(room).await
    }

    async fn fetch_inner(&self, mut req: Request) -> Result<Response> {
        if req.path() == "/socket" {
            return self.socket(req).await;
        }
        let body: Value = req.json().await?;
        let mut room = self.load().await?;
        if req.path() == "/presence" {
            self.prune(&mut room).await?;
            return Response::from_json(
                &room
                    .clients
                    .values()
                    .map(|c| json!({"device_id":c.device_id,"published":c.published}))
                    .collect::<Vec<_>>(),
            );
        }
        if req.path() == "/invalidate" {
            let ids: Vec<String> = room
                .clients
                .iter()
                .filter(|(_, c)| {
                    body.get("all") == Some(&json!(true))
                        || body.get("token").and_then(Value::as_str) == Some(c.token.as_str())
                        || body.get("user_id").and_then(Value::as_i64) == Some(c.user_id)
                })
                .map(|(id, _)| id.clone())
                .collect();
            for id in ids {
                self.remove(&mut room, &id, "Group or device configuration changed.");
            }
            self.prune_state(&mut room).await?;
            self.broadcast(&room).await?;
            self.save(&room).await?;
            return Ok(Response::from_json(
                &json!({"ok":true,"cleanup_pending":!room.cleanup.is_empty() || room.cleanup_in_flight.is_some()}),
            )?
            .with_status(if room.cleanup.is_empty() && room.cleanup_in_flight.is_none() { 200 } else { 202 }));
        }
        let token = body.get("token").and_then(Value::as_str).unwrap_or("");
        let db = self.env.d1("dashboard")?;
        let observe = body
            .get("observe")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let requested = body.get("group_id").and_then(Value::as_str).unwrap_or("");
        let repo = WalkieRepository(&db);
        let access = if observe {
            repo.device_member(requested, token).await
        } else {
            repo.device_group(token).await
        };
        let (group, _uid) = match access {
            Ok(v) => v,
            Err(e) => return e.to_response(),
        };
        if !room.group_id.is_empty() && room.group_id != group.id {
            return Response::error("Invalid group", 403);
        }
        room.group_id = group.id.clone();
        if req.path() == "/ticket" {
            if !observe
                && (self.env.var("WALKIE_SFU_APP_ID").is_err()
                    || self.env.secret("WALKIE_SFU_APP_SECRET").is_err())
            {
                return crate::models::error::AppError::service_unavailable(
                    "Walkie-talkie audio is not configured. Set the backend SFU app ID and secret.",
                )
                .to_response();
            }
            let repo = WalkieRepository(&db);
            if let Err(e) = repo.rate_limit(&format!("connect:{}", token), 120).await {
                return e.to_response();
            }
            let ticket = match random_id() {
                Ok(t) => t,
                Err(e) => return e.to_response(),
            };
            // One outstanding ticket per device prevents unbounded storage on reconnects.
            room.tickets
                .retain(|_, t| (t.token != token || t.observe != observe) && t.expires_at > now());
            room.tickets.insert(
                ticket.clone(),
                Ticket {
                    observe,
                    token: token.into(),
                    group_id: group.id.clone(),
                    expires_at: now() + 30_000,
                },
            );
            self.save(&room).await?;
            return Response::from_json(
                &json!({"ticket":ticket,"group_id":group.id,"group_name":group.name}),
            );
        }
        if req.path() == "/media" {
            let data = body.get("body").cloned().unwrap_or(json!({}));
            let connection = data
                .get("connection_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !room
                .clients
                .get(connection)
                .is_some_and(|c| c.token == token && !c.observe)
            {
                return Response::error("Audio session ended", 403);
            }
            let result = self.media(&mut room, connection, &data).await;
            return match result {
                Ok(v) => Response::from_json(&v),
                Err(e) => e.to_response(),
            };
        }
        Response::error("Not found", 404)
    }

    async fn socket(&self, req: Request) -> Result<Response> {
        let ticket = req
            .url()?
            .query_pairs()
            .find(|(k, _)| k == "ticket")
            .map(|(_, v)| v.into_owned())
            .unwrap_or_default();
        let mut room = self.load().await?;
        let Some(t) = room.tickets.remove(&ticket) else {
            return Response::error("Ticket expired", 401);
        };
        self.save(&room).await?;
        let token = t.token.clone();
        if t.expires_at < now() {
            return Response::error("Ticket expired", 401);
        }
        let db = self.env.d1("dashboard")?;
        let repo = WalkieRepository(&db);
        let access = if t.observe {
            repo.device_member(&t.group_id, &token).await
        } else {
            repo.device_group(&token).await
        };
        let (group, uid) = match access {
            Ok(v) => v,
            Err(e) => return e.to_response(),
        };
        if group.id != t.group_id {
            return Response::error("Assignment changed", 403);
        }
        let previous = room
            .clients
            .iter()
            .filter(|(_, c)| c.token == token && c.observe == t.observe)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in previous {
            self.remove(&mut room, &id, "This device connected in another tab.");
        }
        let id = random_id().map_err(|e| Error::RustError(e.message))?;
        let device_id = repo
            .public_id(&token)
            .await
            .map_err(|e| Error::RustError(e.message))?;
        let profile = repo
            .profile(&token)
            .await
            .map_err(|e| Error::RustError(e.message))?;
        room.group_id = group.id;
        room.clients.insert(
            id.clone(),
            Client {
                device_id: device_id.clone(),
                observe: t.observe,
                user_id: uid,
                token,
                name: profile.first_name.clone(),
                first_name: profile.first_name,
                picture_url: profile.picture_url,
                device_name: profile.device_name,
                last_seen: now(),
                published: false,
                sessions: Vec::new(),
            },
        );
        let pair = WebSocketPair::new()?;
        pair.server.serialize_attachment(&Attachment {
            connection_id: id.clone(),
        })?;
        self.state.accept_web_socket(&pair.server);
        self.save(&room).await?;
        pair.server.send_with_str(
            json!({"type":"connected","connection_id":id,"device_id":device_id,"user_id":uid})
                .to_string(),
        )?;
        self.broadcast(&room).await?;
        self.save(&room).await?;
        Response::from_websocket(pair.client)
    }

    async fn message(&self, ws: WebSocket, message: WebSocketIncomingMessage) -> Result<()> {
        let Some(a) = ws.deserialize_attachment::<Attachment>()? else {
            return Ok(());
        };
        let WebSocketIncomingMessage::String(text) = message else {
            return Ok(());
        };
        if text.len() > 2048 {
            return Ok(());
        }
        let Ok(data) = serde_json::from_str::<Value>(&text) else {
            return Ok(());
        };
        let mut room = self.load().await?;
        // An event already received from this socket proves it is still alive.
        if let Some(client) = room.clients.get_mut(&a.connection_id) {
            client.last_seen = now();
        }
        self.prune(&mut room).await?;
        let Some(c) = room.clients.get_mut(&a.connection_id) else {
            return Ok(());
        };
        c.last_seen = now();
        let uid = c.user_id;
        let published = c.published;
        match data.get("type").and_then(Value::as_str) {
            Some("press") if published => {
                if room.speaker.is_none() {
                    let lease = random_id().map_err(|e| Error::RustError(e.message))?;
                    room.speaker = Some(Speaker {
                        connection_id: a.connection_id.clone(),
                        user_id: uid,
                        lease_id: lease.clone(),
                        expires_at: now() + 6000,
                    });
                    self.save(&room).await?;
                    self.send(&a.connection_id,&json!({"type":"granted","lease_id":lease,"expires_at":now()+6000,"request_id":data.get("request_id")}));
                } else {
                    self.send(&a.connection_id, &json!({"type":"busy"}));
                }
            }
            Some("renew") | Some("release") => {
                let valid = room.speaker.as_ref().is_some_and(|s| {
                    s.connection_id == a.connection_id
                        && data.get("lease_id").and_then(Value::as_str) == Some(s.lease_id.as_str())
                });
                if valid {
                    if data.get("type").and_then(Value::as_str) == Some("release") {
                        room.speaker = None;
                    } else if let Some(s) = &mut room.speaker {
                        s.expires_at = now() + 6000;
                        self.send(&a.connection_id,&json!({"type":"renewed","lease_id":s.lease_id,"expires_at":s.expires_at}));
                    }
                }
            }
            Some("heartbeat") => {}
            _ => {}
        }
        self.save(&room).await?;
        self.broadcast(&room).await?;
        if !room.cleanup.is_empty() {
            self.save(&room).await?;
        }
        Ok(())
    }

    async fn disconnected(&self, ws: WebSocket) -> Result<()> {
        if let Some(a) = ws.deserialize_attachment::<Attachment>()? {
            let mut room = self.load().await?;
            self.remove(&mut room, &a.connection_id, "Disconnected.");
            self.broadcast(&room).await?;
            self.save(&room).await?;
        }
        Ok(())
    }
}

impl DurableObject for WalkieGroupObject {
    fn new(state: State, env: Env) -> Self {
        Self {
            state: std::rc::Rc::new(state),
            env,
        }
    }
    async fn fetch(&self, req: Request) -> Result<Response> {
        if req.path() == "/media" {
            return self.fetch_inner(req).await;
        }
        self.state
            .block_concurrency_while({
                let room = self.clone();
                async move { room.fetch_inner(req).await }
            })
            .await
    }
    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        self.state
            .block_concurrency_while({
                let room = self.clone();
                async move { room.message(ws, message).await }
            })
            .await
    }
    async fn websocket_close(
        &self,
        ws: WebSocket,
        _code: usize,
        _reason: String,
        _clean: bool,
    ) -> Result<()> {
        self.state
            .block_concurrency_while({
                let room = self.clone();
                async move { room.disconnected(ws).await }
            })
            .await
    }
    async fn websocket_error(&self, ws: WebSocket, _error: Error) -> Result<()> {
        self.state
            .block_concurrency_while({
                let room = self.clone();
                async move { room.disconnected(ws).await }
            })
            .await
    }
    async fn alarm(&self) -> Result<Response> {
        self.state
            .block_concurrency_while({
                let coordinator = self.clone();
                async move {
                    let mut room = coordinator.load().await?;
                    coordinator.prune_state(&mut room).await?;
                    coordinator.broadcast(&room).await?;
                    coordinator.save(&room).await?;
                    Response::ok("ok")
                }
            })
            .await?;
        self.drain_cleanup()
            .await
            .map_err(|error| Error::RustError(error.message))?;
        Response::ok("ok")
    }
}
