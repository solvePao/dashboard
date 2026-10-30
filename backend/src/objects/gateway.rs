#[cfg(not(any(test, coverage)))]
use worker::*;
#[cfg(not(any(test, coverage)))]
use serde::{Deserialize, Serialize};
#[cfg(not(any(test, coverage)))]
use serde_json::{json, Value};
#[cfg(not(any(test, coverage)))]
use crate::models::device::{HubEvent, HubEventRequest, HubSnapshot, HubState};

#[cfg(not(any(test, coverage)))]
const HUB_STORAGE_KEY: &str = "hub";
#[cfg(not(any(test, coverage)))]
const MEDIA_OWNER_KEY: &str = "caller_media_owner";

/// Build the single-socket envelope: `{v, channel, group_id?, ...payload}`.
/// Payload fields are flattened so existing payload contracts are preserved.
pub fn envelope(channel: &str, group_id: Option<&str>, payload: &serde_json::Value) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("v".into(), serde_json::json!(1));
    map.insert("channel".into(), serde_json::json!(channel));
    if let Some(g) = group_id { map.insert("group_id".into(), serde_json::json!(g)); }
    match payload {
        serde_json::Value::Object(obj) => { for (k, v) in obj { map.insert(k.clone(), v.clone()); } }
        other => { map.insert("payload".into(), other.clone()); }
    }
    serde_json::Value::Object(map)
}

/// Best-effort delivery to one client's single gateway socket.
#[cfg(not(any(test, coverage)))]
pub async fn deliver_to_gateway(env: &Env, name: &str, channel: &str, group_id: Option<&str>, payload: &Value) {
    let Ok(ns) = env.durable_object(crate::common::GATEWAY_BINDING_NAME) else { return };
    let Ok(stub) = ns.get_by_name(name) else { return };
    let body = json!({"channel": channel, "group_id": group_id, "payload": payload});
    let headers = Headers::new();
    let _ = headers.set("content-type", "application/json");
    let mut init = RequestInit::new();
    init.method = Method::Post;
    init.headers = headers;
    init.body = Some(body.to_string().into());
    let Ok(req) = Request::new_with_init("https://gateway/deliver", &init) else { return };
    let _ = stub.fetch_with_request(req).await;
}

#[cfg(not(any(test, coverage)))]
#[derive(Clone, Default, Serialize, Deserialize)]
struct WalkieSub { group_id: String, connection_id: String, #[serde(default)] observe: bool, #[serde(default)] protocol_version: u8, #[serde(default)] pending_id: String, #[serde(default)] reservation_id: String }

#[cfg(not(any(test, coverage)))]
#[derive(Clone, Serialize, Deserialize)]
struct MediaOwner {
    socket_id: String,
    group_id: String,
    cancelled: bool,
    #[serde(default)] reservation_id: String,
    #[serde(default)] created_at: u64,
    #[serde(default)] orphaned_at: u64,
}

#[cfg(not(any(test, coverage)))]
#[derive(Clone, Default, Serialize, Deserialize)]
struct GatewayAttachment {
    kind: String,
    id: String,
    #[serde(default)]
    socket_id: String,
    #[serde(default)]
    closed: bool,
    #[serde(default)]
    walkie: Vec<WalkieSub>,
}

/// The only Durable Object that accepts client WebSockets.
/// One instance per client (`device:<token>`, `user:<id>`, `pairing:<session>`).
/// All features are channels on this one socket; see `liveSocket.ts`.
#[cfg(not(any(test, coverage)))]
#[durable_object]
pub struct ClientGatewayObject {
    state: State,
    env: Env,
}

#[cfg(not(any(test, coverage)))]
impl ClientGatewayObject {
    fn send_all(&self, value: &Value) {
        let msg = value.to_string();
        for ws in self.state.get_websockets() { let _ = ws.send_with_str(&msg); }
    }

    fn broadcast_media_owner(&self, owner: Option<&MediaOwner>) {
        self.send_all(&envelope("device", None, &json!({"status":"caller_ownership", "busy":owner.is_some(), "group_id":owner.map(|o| &o.group_id), "owner_id":owner.map(|o| &o.socket_id)})));
    }

    async fn room_fetch(&self, group: &str, path: &str, body: &Value) -> Result<Response> {
        let ns = self.env.durable_object("WALKIE_GROUP")?;
        let stub = ns.get_by_name(group)?;
        let headers = Headers::new();
        headers.set("content-type", "application/json")?;
        let mut init = RequestInit::new();
        init.method = Method::Post;
        init.headers = headers;
        init.body = Some(body.to_string().into());
        let req = Request::new_with_init(&format!("https://walkie{path}"), &init)?;
        stub.fetch_with_request(req).await
    }

    async fn handle_deliver(&self, mut req: Request) -> Result<Response> {
        let body: Value = req.json().await.unwrap_or(json!({}));
        let channel = body.get("channel").and_then(Value::as_str).unwrap_or("system").to_string();
        let group_id = body.get("group_id").and_then(Value::as_str).map(str::to_string);
        let payload = body.get("payload").cloned().unwrap_or(json!({}));
        let msg = envelope(&channel, group_id.as_deref(), &payload);
        let text = msg.to_string();
        let is_revoke = channel == "device" && payload.get("status").and_then(Value::as_str) == Some("revoked");
        for ws in self.state.get_websockets() {
            let _ = ws.send_with_str(&text);
            if is_revoke { let _ = ws.close(Some(1000), Some("device revoked")); }
        }
        Response::from_json(&json!({"ok": true}))
    }

    async fn handle_hub_event(&self, mut req: Request) -> Result<Response> {
        let body: HubEventRequest = match req.json().await { Ok(b) => b, Err(_) => return Response::error("Invalid JSON body.", 400) };
        let mut hub: HubState = self.state.storage().get(HUB_STORAGE_KEY).await?.unwrap_or_default();
        hub.revision = hub.revision.saturating_add(1);
        let event = HubEvent { kind: body.kind, token: body.token, revision: hub.revision, at: Date::now().as_millis() };
        hub.last_event = Some(event.clone());
        self.state.storage().put(HUB_STORAGE_KEY, &hub).await?;
        if let Ok(payload) = serde_json::to_value(&event) { self.send_all(&envelope("hub", None, &payload)); }
        Response::from_json(&event)
    }

    async fn handle_socket(&self, req: Request) -> Result<Response> {
        let kind = req.headers().get("x-gateway-kind")?.unwrap_or_default();
        let id = req.headers().get("x-gateway-id")?.unwrap_or_default();
        let pair = WebSocketPair::new()?;
        let socket_id = crate::walkie::random_id().map_err(|e| Error::RustError(e.message))?;
        pair.server.serialize_attachment(&GatewayAttachment { kind: kind.clone(), id, socket_id: socket_id.clone(), closed: false, walkie: Vec::new() })?;
        self.state.accept_web_socket(&pair.server);
        pair.server.send_with_str(envelope("system", None, &json!({"type":"hello","kind":kind,"gateway_socket_id":socket_id})).to_string())?;
        if kind == "device" {
            let owner = self.reconcile_media_owner().await?;
            pair.server.send_with_str(envelope("device", None, &json!({"status":"caller_ownership", "busy":owner.is_some(), "group_id":owner.as_ref().map(|o| &o.group_id), "owner_id":owner.as_ref().map(|o| &o.socket_id)})).to_string())?;
        }
        if kind == "user" {
            let hub: HubState = self.state.storage().get(HUB_STORAGE_KEY).await?.unwrap_or_default();
            let snap = HubSnapshot { kind: "snapshot".into(), revision: hub.revision, last_event: hub.last_event };
            if let Ok(payload) = serde_json::to_value(&snap) { let _ = pair.server.send_with_str(envelope("hub", None, &payload).to_string()); }
        }
        Response::from_websocket(pair.client)
    }

    async fn walkie_subscribe(&self, ws: &WebSocket, att: &mut GatewayAttachment, data: &Value) -> Result<()> {
        let group = data.get("group_id").and_then(Value::as_str).unwrap_or("").to_string();
        if att.kind != "device" || group.is_empty() {
            let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"error","message":"Walkie is available on authorized devices only."})).to_string());
            return Ok(());
        }
        if data.get("protocol_version").and_then(Value::as_u64) != Some(2) || att.socket_id.is_empty() {
            let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"error","code":"refresh_required","message":"Caller was updated. Refresh this page to continue."})).to_string());
            return Ok(());
        }
        let observe = data.get("observe").and_then(Value::as_bool).unwrap_or(false);
        let mut pending_id = String::new();
        let mut reservation_id = String::new();
        if observe {
            if let Some(current) = ws.deserialize_attachment::<GatewayAttachment>()? { *att = current; }
            if att.closed || att.walkie.iter().any(|s| s.group_id == group && s.observe) { return Ok(()); }
            pending_id = crate::walkie::random_id().map_err(|e| Error::RustError(e.message))?;
            att.walkie.push(WalkieSub { group_id: group.clone(), connection_id: String::new(), observe: true, protocol_version: 2, pending_id: pending_id.clone(), reservation_id: String::new() });
            ws.serialize_attachment(&*att)?;
        }
        if !observe {
            reservation_id = crate::walkie::random_id().map_err(|e| Error::RustError(e.message))?;
            let owner = MediaOwner { socket_id: att.socket_id.clone(), group_id: group.clone(), cancelled: false, reservation_id: reservation_id.clone(), created_at: Date::now().as_millis(), orphaned_at: 0 };
            if !self.reserve_media_owner(owner.clone()).await? {
                let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"error","code":"device_busy","message":"Caller audio is already active in another group or tab."})).to_string());
                return Ok(());
            }
            self.broadcast_media_owner(Some(&owner));
            if ws.deserialize_attachment::<GatewayAttachment>()?.is_none_or(|current| current.closed) {
                self.release_media_owner(&reservation_id).await?;
                return Ok(());
            }
        }
        let mut res = match self.room_fetch(&group, "/gateway-join", &json!({"token": att.id, "observe": observe, "group_id": group, "protocol_version": 2})).await {
            Ok(r) => r, Err(_) => {
                if !observe { self.release_media_owner(&reservation_id).await?; let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"unsubscribed","observe":false})).to_string()); }
                else { self.clear_pending_observer(ws, &pending_id)?; }
                let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"error","message":"Caller coordinator unavailable."})).to_string()); return Ok(());
            }
        };
        let status = res.status_code();
        let body: Value = res.json().await.unwrap_or(json!({}));
        if !(200..300).contains(&status) {
            if !observe { self.release_media_owner(&reservation_id).await?; let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"unsubscribed","observe":false})).to_string()); }
            else { self.clear_pending_observer(ws, &pending_id)?; }
            let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"error","message": body.get("error").and_then(Value::as_str).unwrap_or("Could not join group.")})).to_string());
            return Ok(());
        }
        let connection_id = body.get("connection_id").and_then(Value::as_str).unwrap_or("").to_string();
        if observe {
            let current = ws.deserialize_attachment::<GatewayAttachment>()?;
            if !current.as_ref().is_some_and(|a| !a.closed && a.walkie.iter().any(|s| s.pending_id == pending_id)) {
                let _ = self.room_fetch(&group, "/gateway-leave", &json!({"token": att.id, "connection_id": connection_id, "message": {"type":"unsubscribe"}})).await;
                return Ok(());
            }
        }
        if !observe {
            let owner: Option<MediaOwner> = self.state.storage().get(MEDIA_OWNER_KEY).await?;
            if !owner.as_ref().is_some_and(|o| o.reservation_id == reservation_id && !o.cancelled) {
                let _ = self.room_fetch(&group, "/gateway-leave", &json!({"token": att.id, "connection_id": connection_id, "message": {"type":"unsubscribe"}})).await;
                self.release_media_owner(&reservation_id).await?;
                let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"unsubscribed","observe":false})).to_string());
                return Ok(());
            }
        }
        if let Some(current) = ws.deserialize_attachment::<GatewayAttachment>()? { *att = current; }
        att.walkie.retain(|s| !(s.group_id == group && s.observe == observe));
        att.walkie.push(WalkieSub { group_id: group.clone(), connection_id: connection_id.clone(), observe, protocol_version: 2, pending_id: String::new(), reservation_id });
        let _ = ws.serialize_attachment(att);
        let mut reply = body.clone();
        if let Some(o) = reply.as_object_mut() {
            o.insert("type".into(), json!("connected"));
            // Demux key: a media session and an observe subscription for the
            // same group share this socket; only media may claim connection_id.
            o.insert("observe".into(), json!(observe));
        }
        let _ = ws.send_with_str(envelope("walkie", Some(&group), &reply).to_string());
        Ok(())
    }

    async fn walkie_forward(&self, ws: &WebSocket, att: &GatewayAttachment, data: &Value) -> Result<bool> {
        let group = data.get("group_id").and_then(Value::as_str).unwrap_or("");
        // Demux by (group, observe): the media session (observe=false) and the
        // activity observer (observe=true) share this socket, and signals such
        // as press/call_start only mean anything on the media connection.
        let observe = data.get("observe").and_then(Value::as_bool).unwrap_or(false);
        let typ = data.get("type").and_then(Value::as_str).unwrap_or("");
        let Some(sub) = att.walkie.iter().find(|s| s.group_id == group && s.observe == observe && s.protocol_version == 2 && s.pending_id.is_empty()) else {
            if typ == "unsubscribe" && !observe {
                if !self.cancel_media_owner(&att.socket_id, Some(group)).await? {
                    let _ = ws.send_with_str(envelope("walkie", Some(group), &json!({"type":"unsubscribed","observe":false})).to_string());
                }
            } else if typ == "unsubscribe" {
                let _ = ws.send_with_str(envelope("walkie", Some(group), &json!({"type":"unsubscribed","observe":true})).to_string());
            }
            return Ok(true);
        };
        if observe && !matches!(typ, "unsubscribe" | "call_decline" | "call_end") { return Ok(false); }
        if !observe {
            let owner: Option<MediaOwner> = self.state.storage().get(MEDIA_OWNER_KEY).await?;
            if !owner.is_some_and(|o| o.reservation_id == sub.reservation_id && o.socket_id == att.socket_id && !o.cancelled) { return Ok(false); }
        }
        let path = if typ == "unsubscribe" { "/gateway-leave" } else { "/gateway-signal" };
        let mut message = data.clone();
        if let Some(fields) = message.as_object_mut() { fields.insert("protocol_version".into(), json!(2)); }
        let result = self.room_fetch(group, path, &json!({"token": att.id, "connection_id": sub.connection_id, "message": message})).await;
        if typ == "unsubscribe" && !result.as_ref().is_ok_and(|r| (200..300).contains(&r.status_code()) || r.status_code() == 403) {
            let _ = ws.send_with_str(envelope("walkie", Some(group), &json!({"type":"error","code":"unsubscribe_failed","message":"Could not leave the call. Try again."})).to_string());
            return Ok(false);
        }
        if typ == "unsubscribe" && !observe {
            self.release_media_owner(&sub.reservation_id).await?;
            let _ = ws.send_with_str(envelope("walkie", Some(group), &json!({"type":"unsubscribed","observe":false})).to_string());
        }
        if typ == "unsubscribe" && observe {
            let _ = ws.send_with_str(envelope("walkie", Some(group), &json!({"type":"unsubscribed","observe":true})).to_string());
        }
        Ok(true)
    }

    fn clear_pending_observer(&self, ws: &WebSocket, pending_id: &str) -> Result<()> {
        if let Some(mut att) = ws.deserialize_attachment::<GatewayAttachment>()? {
            att.walkie.retain(|s| s.pending_id != pending_id);
            ws.serialize_attachment(&att)?;
        }
        Ok(())
    }

    async fn reconcile_media_owner(&self) -> Result<Option<MediaOwner>> {
        let sockets = self.state.get_websockets();
        let storage = self.state.storage();
        let now = Date::now().as_millis();
        let (owner, delay, removed) = self.state.block_concurrency_while(async move {
            let Some(mut owner) = storage.get::<MediaOwner>(MEDIA_OWNER_KEY).await? else { return Ok((None, None, false)) };
            // Inspect attachments inside the same gate as the ownership update:
            // a completed join must not be expired using a pre-join snapshot.
            let mut socket_present = false;
            let mut media_attached = false;
            for ws in sockets {
                if let Ok(Some(att)) = ws.deserialize_attachment::<GatewayAttachment>() {
                    if att.socket_id == owner.socket_id && !att.closed {
                        socket_present = true;
                        media_attached = att.walkie.iter().any(|sub| !sub.observe && sub.reservation_id == owner.reservation_id && !sub.connection_id.is_empty());
                        break;
                    }
                }
            }
            if socket_present && media_attached {
                if owner.orphaned_at != 0 { owner.orphaned_at = 0; storage.put(MEDIA_OWNER_KEY, &owner).await?; }
                return Ok((Some(owner), None, false));
            }
            let deadline = if socket_present {
                owner.created_at.saturating_add(60_000)
            } else {
                if owner.orphaned_at == 0 { owner.orphaned_at = now; storage.put(MEDIA_OWNER_KEY, &owner).await?; }
                owner.orphaned_at.saturating_add(60_000)
            };
            if deadline <= now {
                storage.delete(MEDIA_OWNER_KEY).await?;
                Ok((None, None, true))
            } else {
                Ok((Some(owner), Some(deadline - now), false))
            }
        }).await?;
        if removed { self.broadcast_media_owner(None); }
        if let Some(delay) = delay { self.state.storage().set_alarm(std::time::Duration::from_millis(delay.max(1))).await?; }
        Ok(owner)
    }

    async fn reserve_media_owner(&self, owner: MediaOwner) -> Result<bool> {
        self.reconcile_media_owner().await?;
        let storage = self.state.storage();
        let reserved = self.state.block_concurrency_while(async move {
            let current: Option<MediaOwner> = storage.get(MEDIA_OWNER_KEY).await?;
            if current.is_some() { return Ok(false); }
            storage.put(MEDIA_OWNER_KEY, owner).await?;
            Ok(true)
        }).await?;
        if reserved { self.state.storage().set_alarm(std::time::Duration::from_millis(60_000)).await?; }
        Ok(reserved)
    }

    async fn cancel_media_owner(&self, socket_id: &str, group: Option<&str>) -> Result<bool> {
        let storage = self.state.storage();
        let socket_id = socket_id.to_string();
        let group = group.map(str::to_string);
        self.state.block_concurrency_while(async move {
            let owner: Option<MediaOwner> = storage.get(MEDIA_OWNER_KEY).await?;
            if let Some(mut owner) = owner.filter(|o| o.socket_id == socket_id && group.as_ref().is_none_or(|g| o.group_id == *g)) {
                owner.cancelled = true;
                storage.put(MEDIA_OWNER_KEY, owner).await?;
                return Ok(true);
            }
            Ok(false)
        }).await
    }

    async fn release_media_owner(&self, reservation_id: &str) -> Result<()> {
        let storage = self.state.storage();
        let reservation_id = reservation_id.to_string();
        let removed = self.state.block_concurrency_while(async move {
            let owner: Option<MediaOwner> = storage.get(MEDIA_OWNER_KEY).await?;
            if !reservation_id.is_empty() && owner.is_some_and(|o| o.reservation_id == reservation_id) {
                storage.delete(MEDIA_OWNER_KEY).await?;
                return Ok(true);
            }
            Ok(false)
        }).await?;
        if removed { self.broadcast_media_owner(None); }
        Ok(())
    }

    async fn leave_all(&self, att: &GatewayAttachment) {
        let _ = self.cancel_media_owner(&att.socket_id, None).await;
        for sub in &att.walkie {
            if !sub.pending_id.is_empty() { continue; }
            let _ = self.room_fetch(&sub.group_id, "/gateway-leave", &json!({"token": att.id, "connection_id": sub.connection_id, "message": {"type":"unsubscribe"}})).await;
            if !sub.observe { let _ = self.release_media_owner(&sub.reservation_id).await; }
        }
    }
}

#[cfg(not(any(test, coverage)))]
impl DurableObject for ClientGatewayObject {
    fn new(state: State, env: Env) -> Self { Self { state, env } }

    async fn fetch(&self, req: Request) -> Result<Response> {
        let is_socket = req.headers().get("upgrade")?.map(|v| v.eq_ignore_ascii_case("websocket")).unwrap_or(false);
        match (req.method(), req.path().as_str()) {
            (Method::Post, "/deliver") => self.handle_deliver(req).await,
            (Method::Post, "/hub-event") => self.handle_hub_event(req).await,
            (Method::Get, _) if is_socket => self.handle_socket(req).await,
            _ => Response::error("Not found.", 404),
        }
    }

    async fn websocket_message(&self, ws: WebSocket, message: WebSocketIncomingMessage) -> Result<()> {
        let WebSocketIncomingMessage::String(text) = message else { return Ok(()) };
        if text.len() > 4096 { return Ok(()); }
        let Ok(data) = serde_json::from_str::<Value>(&text) else { return Ok(()) };
        let Some(mut att) = ws.deserialize_attachment::<GatewayAttachment>()? else { return Ok(()) };
        match data.get("channel").and_then(Value::as_str) {
            Some("walkie") => {
                match data.get("type").and_then(Value::as_str) {
                    Some("subscribe") => self.walkie_subscribe(&ws, &mut att, &data).await?,
                    Some("unsubscribe") => {
                        let observe = data.get("observe").and_then(Value::as_bool).unwrap_or(false);
                        let previous = att.walkie.iter().find(|s| Some(s.group_id.as_str()) == data.get("group_id").and_then(Value::as_str) && s.observe == observe).cloned();
                        if self.walkie_forward(&ws, &att, &data).await? {
                            if let Some(current) = ws.deserialize_attachment::<GatewayAttachment>()? { att = current; }
                            if let Some(previous) = previous {
                                att.walkie.retain(|s| !(s.group_id == previous.group_id && s.observe == previous.observe && s.connection_id == previous.connection_id && s.pending_id == previous.pending_id));
                            }
                            let _ = ws.serialize_attachment(&att);
                        }
                    }
                    _ => { self.walkie_forward(&ws, &att, &data).await?; }
                }
            }
            Some("pairing") => {
                if att.kind == "pairing" {
                    if let Ok(ns) = self.env.durable_object(crate::common::PAIRING_BINDING_NAME) {
                        if let Ok(stub) = ns.get_by_name(&att.id) {
                            let req = Request::new("https://pairing/status", Method::Get)?;
                            if let Ok(mut res) = stub.fetch_with_request(req).await {
                                if res.status_code() == 200 {
                                    if let Ok(payload) = res.json::<Value>().await {
                                        let _ = ws.send_with_str(envelope("pairing", None, &payload).to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // Single heartbeat keeps every walkie subscription alive.
                if data.get("type").and_then(Value::as_str) == Some("heartbeat") {
                    for sub in att.walkie.clone() {
                        if sub.protocol_version != 2 { continue; }
                        let _ = self.room_fetch(&sub.group_id, "/gateway-signal", &json!({"token": att.id, "connection_id": sub.connection_id, "message": {"type":"heartbeat"}})).await;
                    }
                }
            }
        }
        Ok(())
    }

    async fn websocket_close(&self, ws: WebSocket, _code: usize, _reason: String, _was_clean: bool) -> Result<()> {
        if let Some(mut att) = ws.deserialize_attachment::<GatewayAttachment>()? { att.closed = true; let _ = ws.serialize_attachment(&att); self.leave_all(&att).await; }
        Ok(())
    }

    async fn websocket_error(&self, ws: WebSocket, _error: Error) -> Result<()> {
        if let Some(mut att) = ws.deserialize_attachment::<GatewayAttachment>()? { att.closed = true; let _ = ws.serialize_attachment(&att); self.leave_all(&att).await; }
        Ok(())
    }

    async fn alarm(&self) -> Result<Response> {
        self.reconcile_media_owner().await?;
        Response::ok("ok")
    }
}
