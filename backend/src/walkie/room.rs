mod media;

use super::{repository::WalkieRepository, sfu::SfuService, *};
use futures_util::future::join_all;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[derive(Clone, Default, Serialize, Deserialize)]
struct Room {
    group_id: String,
    clients: BTreeMap<String, Client>,
    cleanup: Vec<MediaSession>,
    #[serde(default)]
    cleanup_in_flight: Option<MediaSession>,
    #[serde(default)]
    cleanup_started_at: u64,
    #[serde(default)]
    tickets: BTreeMap<String, Ticket>,
    #[serde(default)]
    call: Option<Call>,
    #[serde(default)]
    caller_version: u8,
}
#[derive(Clone, Serialize, Deserialize)]
struct Call {
    call_id: String,
    started_by: String,
    participants: Vec<String>,
    started_at: u64,
    ringing_until: u64,
    #[serde(default)]
    established: bool,
    #[serde(default)]
    declined: Vec<String>,
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
        let mut room: Room = self.state.storage().get("room").await?.unwrap_or_default();
        if room.caller_version != 2 {
            // Legacy connections could auto-join. Tear down their transient media,
            // while preserving cleanup and all externally stored group settings.
            for client in std::mem::take(&mut room.clients).into_values() {
                room.cleanup.extend(client.sessions);
            }
            room.call = None;
            room.tickets.clear();
            room.caller_version = 2;
        }
        Ok(room)
    }

    async fn save(&self, room: &Room) -> Result<()> {
        self.state.storage().put("room", room).await?;
        self.schedule_alarm(room).await
    }

    async fn schedule_alarm(&self, room: &Room) -> Result<()> {
        let storage = self.state.storage();
        let existing = storage.get_alarm().await?;
        if !room.clients.is_empty() || !room.cleanup.is_empty()
            || room.cleanup_in_flight.is_some() || !room.tickets.is_empty() || room.call.is_some()
        {
            let cleanup_delay = if room.cleanup.is_empty() || room.cleanup_in_flight.is_some() { 15_000 } else { 1000 };
            let current = now();
            let deadline = room
                .call
                .as_ref()
                .filter(|c| !c.established)
                .map(|c| c.ringing_until.min(current + cleanup_delay))
                .unwrap_or(current + cleanup_delay);
            // Active traffic must not postpone revocation and expiration checks.
            if existing.is_none_or(|time| time as u64 > deadline) {
                storage
                    .set_alarm(Duration::from_millis(deadline.saturating_sub(now()).max(1)))
                    .await?;
            }
        } else if existing.is_some() {
            storage.delete_alarm().await?;
        }
        Ok(())
    }

    /// Deliver one payload to one room client via its single gateway socket.
    async fn deliver(&self, room: &Room, id: &str, value: &Value) {
        if let Some(client) = room.clients.get(id) {
            crate::objects::gateway::deliver_to_gateway(
                &self.env,
                &crate::common::gateway_device_name(&client.token),
                "walkie",
                Some(&room.group_id),
                value,
            )
            .await;
        }
    }

    fn call_json(room: &Room) -> Value {
        match &room.call {
            Some(c) => json!({"call_id": c.call_id, "started_by": c.started_by, "participants": c.participants, "started_at": c.started_at, "ringing_until": c.ringing_until, "established": c.established, "declined": c.declined}),
            None => Value::Null,
        }
    }

    async fn broadcast(&self, room: &Room) -> Result<()> {
        let tokens: BTreeSet<_> = room
            .clients
            .values()
            .map(|client| client.token.as_str())
            .collect();
        if tokens.is_empty() {
            return Ok(());
        }
        let db = self.env.d1("dashboard")?;
        let repo = WalkieRepository(&db);
        let (members, group) =
            futures_util::join!(repo.members(&room.group_id), repo.group(&room.group_id));
        let mut members = members.unwrap_or_default();
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
        let name = group.ok().map(|group| group.name);
        let value = json!({"type":"snapshot","group_name":name,"members":members,
            "clients":room.clients.iter().filter(|(_,c)|!c.observe).map(|(id,c)| json!({"connection_id":id,"device_id":c.device_id,"user_id":c.user_id,"name":c.first_name,"first_name":c.first_name,"picture_url":c.picture_url,"device_name":c.device_name,"published":c.published})).collect::<Vec<_>>(),
            "call": Self::call_json(room)});
        join_all(tokens.into_iter().map(|token| {
            let value = &value;
            async move {
                crate::objects::gateway::deliver_to_gateway(
                    &self.env,
                    &crate::common::gateway_device_name(token),
                    "walkie",
                    Some(&room.group_id),
                    value,
                )
                .await;
            }
        }))
        .await;
        Ok(())
    }

    async fn remove(&self, room: &mut Room, id: &str, reason: &str) {
        // Capture delivery target before the client leaves the room.
        let target_token = room.clients.get(id).map(|c| c.token.clone());
        Self::remove_client(room, id);
        if let Some(token) = target_token {
            crate::objects::gateway::deliver_to_gateway(&self.env, &crate::common::gateway_device_name(&token), "walkie", Some(&room.group_id), &json!({"type":"ended","connection_id":id,"reason":reason})).await;
        }
    }

    fn remove_client(room: &mut Room, id: &str) {
        if let Some(client) = room.clients.remove(id) {
            room.cleanup.extend(client.sessions);
        }
        Self::leave_participant(room, id);
    }

    async fn prune_state(&self, room: &mut Room) -> Result<bool> {
        let current = now();
        room.tickets.retain(|_, t| t.expires_at > current);
        if room.clients.is_empty() && room.call.is_none() {
            return Ok(false);
        }
        let db = self.env.d1("dashboard")?;
        let repo = WalkieRepository(&db);
        #[derive(Deserialize)]
        struct Authorized {
            token: String,
            user_id: i64,
        }
        // Validate the room in one database read rather than one round trip per check per socket.
        let authorized = repo.rows::<Authorized>(
            "SELECT t.token,t.user_id FROM device_tokens t JOIN walkie_device_members m ON m.token=t.token JOIN device_widgets w ON w.token=t.token AND w.widget_key='walkie_talkie' AND w.enabled=1 WHERE m.group_id=? AND t.expires_at>strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            &[D1Type::Text(&room.group_id)],
        ).await.map_err(|error| Error::RustError(error.message))?;
        let removed: Vec<String> = room
            .clients
            .iter()
            .filter(|(_, client)| {
                !authorized.iter().any(|entry| {
                    entry.token == client.token
                        && entry.user_id == client.user_id
                }) || client.device_id.is_empty()
                    || client.last_seen + 45_000 < current
            })
            .map(|(id, _)| id.clone())
            .collect();
        let mut changed = !removed.is_empty();
        for id in removed {
            self.remove(
                room,
                &id,
                "Access changed or connection timed out. Join again to retry.",
            )
            .await;
        }
        if room
            .call
            .as_ref()
            .is_some_and(|c| !c.established && c.ringing_until <= current)
        {
            Self::end_call(room);
            changed = true;
        }
        Ok(changed)
    }

    async fn refresh_presence(&self, room: &mut Room, from_alarm: bool) -> Result<()> {
        let ticket_count = room.tickets.len();
        let changed = self.prune_state(room).await?;
        if changed || ticket_count != room.tickets.len() {
            self.save(room).await?;
        } else if from_alarm {
            self.schedule_alarm(room).await?;
        }
        // Periodic snapshots also recover missed gateway deliveries and refresh roster metadata.
        if changed || from_alarm {
            self.broadcast(room).await?;
        }
        Ok(())
    }

    async fn fetch_inner(&self, mut req: Request) -> Result<Response> {
        let body: Value = req.json().await?;
        let mut room = self.load().await?;
        let path = req.path();
        if path == "/presence" {
            self.refresh_presence(&mut room, false).await?;
            return Response::from_json(
                &room
                    .clients
                    .values()
                    .map(|c| json!({"device_id":c.device_id,"published":c.published}))
                    .collect::<Vec<_>>(),
            );
        }
        if path == "/invalidate" {
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
                self.remove(&mut room, &id, "Group or device configuration changed.").await;
            }
            self.prune_state(&mut room).await?;
            self.save(&room).await?;
            self.broadcast(&room).await?;
            return Ok(Response::from_json(
                &json!({"ok":true,"cleanup_pending":!room.cleanup.is_empty() || room.cleanup_in_flight.is_some()}),
            )?
            .with_status(if room.cleanup.is_empty() && room.cleanup_in_flight.is_none() { 200 } else { 202 }));
        }
        let token = body.get("token").and_then(Value::as_str).unwrap_or("");
        let connection = body
            .get("connection_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        let message = body.get("message").unwrap_or(&Value::Null);
        let requested = body
            .get("group_id")
            .and_then(Value::as_str)
            .unwrap_or(&room.group_id);
        if !room.group_id.is_empty() && room.group_id != requested {
            return Response::error("Invalid group", 403);
        }

        // The gateway can only renew or remove connections it already owns.
        // The room alarm revalidates device access even while heartbeats arrive.
        if path == "/gateway-signal"
            && message.get("type").and_then(Value::as_str) == Some("heartbeat")
        {
            let Some(client) = room
                .clients
                .get_mut(connection)
                .filter(|client| client.token == token)
            else {
                return Response::error("Audio session ended", 403);
            };
            client.last_seen = now();
            self.save(&room).await?;
            return Response::from_json(&json!({"ok":true}));
        }
        if path == "/gateway-leave" {
            if !room
                .clients
                .get(connection)
                .is_some_and(|client| client.token == token)
            {
                return Response::error("Audio session ended", 403);
            }
            Self::remove_client(&mut room, connection);
            self.save(&room).await?;
            self.broadcast(&room).await?;
            return Response::from_json(&json!({"ok":true}));
        }

        // These private routes are reached only after the public Worker has
        // checked the token, widget, and membership for this group.
        if path == "/call-state" {
            self.refresh_presence(&mut room, false).await?;
            return Response::from_json(&json!({"call":Self::call_json(&room)}));
        }
        if path == "/media" {
            let data = body.get("body").unwrap_or(&Value::Null);
            let connection = data
                .get("connection_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !room
                .clients
                .get(connection)
                .is_some_and(|client| client.token == token && !client.observe)
            {
                return Response::error("Audio session ended", 403);
            }
            return match self.media(&mut room, connection, data).await {
                Ok(value) => Response::from_json(&value),
                Err(error) => error.to_response(),
            };
        }

        let db = self.env.d1("dashboard")?;
        let observe = body
            .get("observe")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let repo = WalkieRepository(&db);
        let access = repo.device_member(requested, token).await;
        let (group, uid) = match access {
            Ok(v) => v,
            Err(e) => return e.to_response(),
        };
        room.group_id = group.id.clone();
        if path == "/gateway-join" {
            if body.get("protocol_version").and_then(Value::as_u64) != Some(2) {
                return Response::error("Refresh this page to use Caller.", 409);
            }
            return self
                .gateway_join(&mut room, token, uid, observe, &group, &repo)
                .await;
        }
        if path == "/gateway-signal" {
            let Some(client) = room
                .clients
                .get_mut(connection)
                .filter(|client| client.token == token)
            else {
                return Response::error("Audio session ended", 403);
            };
            client.last_seen = now();
            let pruned = self.prune_state(&mut room).await?;
            let reply = Self::signal(&mut room, connection, message)?;
            self.save(&room).await?;
            let mut changed = pruned;
            if let Some(reply) = reply {
                changed |= reply["type"] != "error";
                self.deliver(&room, connection, &reply).await;
            }
            if changed {
                self.broadcast(&room).await?;
            }
            return Response::from_json(&json!({"ok":true}));
        }
        if path == "/ticket" {
            if !observe
                && (self.env.var("WALKIE_SFU_APP_ID").is_err()
                    || self.env.secret("WALKIE_SFU_APP_SECRET").is_err())
            {
                return crate::models::error::AppError::service_unavailable(
                    "Walkie-talkie audio is not configured. Set the backend SFU app ID and secret.",
                )
                .to_response();
            }
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
        Response::error("Not found", 404)
    }

    /// Gateway-mediated join: the client socket lives on ClientGatewayObject;
    /// this room only registers the media/presence client. Access was already
    /// validated by `fetch_inner` (`device_member`).
    async fn gateway_join(
        &self,
        room: &mut Room,
        token: &str,
        uid: i64,
        observe: bool,
        group: &Group,
        repo: &WalkieRepository<'_>,
    ) -> Result<Response> {
        if !observe
            && room
                .clients
                .values()
                .any(|c| c.token == token && !c.observe)
        {
            return Response::error("Caller audio is already active in another tab.", 409);
        }
        let id = random_id().map_err(|e| Error::RustError(e.message))?;
        let profile = repo.profile(token).await.map_err(|e| Error::RustError(e.message))?;
        let device_id = profile.device_id;
        room.clients.insert(id.clone(), Client {
            device_id: device_id.clone(), observe, user_id: uid, token: token.into(),
            name: profile.first_name.clone(), first_name: profile.first_name,
            picture_url: profile.picture_url, device_name: profile.device_name,
            last_seen: now(), published: false, sessions: Vec::new(),
        });
        self.save(room).await?;
        self.broadcast(room).await?;
        Response::from_json(&json!({"connection_id":id,"device_id":device_id,"user_id":uid,"group_id":group.id,"group_name":group.name}))
    }

    fn leave_participant(room: &mut Room, id: &str) {
        let clear = room.call.as_mut().is_some_and(|call| {
            let was_participant = call.participants.iter().any(|p| p == id);
            call.participants.retain(|p| p != id);
            was_participant && (call.participants.is_empty() || (call.established && call.participants.len() < 2))
        });
        Self::retire_media(room, &[id.to_string()]);
        if clear { Self::end_call(room); }
    }

    fn retire_media(room: &mut Room, connections: &[String]) {
        for (id, client) in &mut room.clients {
            let departing = connections.contains(id);
            if departing { client.published = false; }
            let mut kept = Vec::new();
            for session in std::mem::take(&mut client.sessions) {
                if departing || session.target.as_ref().is_some_and(|target| connections.contains(target)) {
                    room.cleanup.push(session);
                } else { kept.push(session); }
            }
            client.sessions = kept;
        }
    }

    fn end_call(room: &mut Room) {
        if let Some(call) = room.call.take() { Self::retire_media(room, &call.participants); }
    }

    fn signal(room: &mut Room, connection_id: &str, data: &Value) -> Result<Option<Value>> {
        let Some(client) = room.clients.get(connection_id) else {
            return Ok(None);
        };
        let device_id = client.device_id.clone();
        let published = client.published && !client.observe;
        let kind = data.get("type").and_then(Value::as_str).unwrap_or("");
        let mut reply = json!({"type":"error", "connection_id":connection_id,
            "request_id":data.get("request_id"), "message":"This call is no longer available."});
        if data.get("protocol_version").and_then(Value::as_u64) != Some(2) {
            reply["code"] = json!("refresh_required");
            reply["message"] = json!("Refresh this page to use Caller.");
        } else if kind == "call_start" && published {
            if room.call.is_none() {
                let call_id = random_id().map_err(|e| Error::RustError(e.message))?;
                room.call = Some(Call { call_id: call_id.clone(), started_by: device_id,
                    participants: vec![connection_id.into()], started_at: now(),
                    ringing_until: now() + 60_000, established: false, declined: Vec::new() });
                reply["type"] = json!("call_started");
                reply["call_id"] = json!(call_id);
            } else { reply["message"] = json!("This group already has a call. Join it instead."); }
        } else if let Some(call) = room.call.as_mut().filter(|c|
            data.get("call_id").and_then(Value::as_str) == Some(c.call_id.as_str())) {
            let participant = call.participants.iter().any(|p| p == connection_id);
            let starter = call.started_by == device_id && participant;
            let accepted = match kind {
                "call_join" if published => {
                    if !participant { call.participants.push(connection_id.into()); }
                    call.declined.retain(|id| id != &device_id);
                    call.established |= call.participants.len() >= 2;
                    true
                }
                "call_decline" if !participant => {
                    if !call.declined.contains(&device_id) { call.declined.push(device_id); }
                    true
                }
                "call_leave" if participant => true,
                "call_cancel" if starter && !call.established => true,
                "call_end" if starter => true,
                _ => false,
            };
            if accepted {
                reply["type"] = json!("call_ack");
                reply["call_id"] = json!(call.call_id);
                if kind == "call_leave" { Self::leave_participant(room, connection_id); }
                else if kind == "call_end" || kind == "call_cancel" { Self::end_call(room); }
            } else { reply["message"] = json!("This call action is not permitted."); }
        }
        Ok(Some(reply))
    }

    async fn disconnected_id(&self, connection_id: &str) -> Result<()> {
        let mut room = self.load().await?;
        self.remove(&mut room, connection_id, "Disconnected.").await;
        self.save(&room).await?;
        self.broadcast(&room).await?;
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
    // This object accepts no client WebSockets (gateway owns them all);
    // handlers remain only to satisfy the Durable Object contract.
    async fn websocket_message(
        &self,
        _ws: WebSocket,
        _message: WebSocketIncomingMessage,
    ) -> Result<()> {
        Ok(())
    }
    async fn websocket_close(
        &self,
        ws: WebSocket,
        _code: usize,
        _reason: String,
        _clean: bool,
    ) -> Result<()> {
        if let Some(a) = ws.deserialize_attachment::<Attachment>()? {
            let connection_id = a.connection_id.clone();
            return self.state
                .block_concurrency_while({
                    let room = self.clone();
                    async move { room.disconnected_id(&connection_id).await }
                })
                .await;
        }
        Ok(())
    }
    async fn websocket_error(&self, ws: WebSocket, _error: Error) -> Result<()> {
        if let Some(a) = ws.deserialize_attachment::<Attachment>()? {
            let connection_id = a.connection_id.clone();
            return self.state
                .block_concurrency_while({
                    let room = self.clone();
                    async move { room.disconnected_id(&connection_id).await }
                })
                .await;
        }
        Ok(())
    }
    async fn alarm(&self) -> Result<Response> {
        let has_cleanup = self
            .state
            .block_concurrency_while({
                let coordinator = self.clone();
                async move {
                    let mut room = coordinator.load().await?;
                    coordinator.refresh_presence(&mut room, true).await?;
                    Ok(!room.cleanup.is_empty() || room.cleanup_in_flight.is_some())
                }
            })
            .await?;
        if has_cleanup {
            self.drain_cleanup()
                .await
                .map_err(|error| Error::RustError(error.message))?;
        }
        Response::ok("ok")
    }
}
