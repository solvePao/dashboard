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
struct WalkieSub { group_id: String, connection_id: String, #[serde(default)] observe: bool }

#[cfg(not(any(test, coverage)))]
#[derive(Clone, Default, Serialize, Deserialize)]
struct GatewayAttachment {
    kind: String,
    id: String,
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
        let is_ended = channel == "walkie" && payload.get("type").and_then(Value::as_str) == Some("ended");
        for ws in self.state.get_websockets() {
            let _ = ws.send_with_str(&text);
            if is_ended {
                if let Ok(Some(mut att)) = ws.deserialize_attachment::<GatewayAttachment>() {
                    if let Some(g) = &group_id { att.walkie.retain(|s| &s.group_id != g); let _ = ws.serialize_attachment(&att); }
                }
            }
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
        pair.server.serialize_attachment(&GatewayAttachment { kind: kind.clone(), id, walkie: Vec::new() })?;
        self.state.accept_web_socket(&pair.server);
        pair.server.send_with_str(envelope("system", None, &json!({"type":"hello","kind":kind})).to_string())?;
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
        let observe = data.get("observe").and_then(Value::as_bool).unwrap_or(false);
        let mut res = match self.room_fetch(&group, "/gateway-join", &json!({"token": att.id, "observe": observe, "group_id": group})).await {
            Ok(r) => r, Err(_) => { let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"error","message":"Walkie coordinator unavailable."})).to_string()); return Ok(()); }
        };
        let status = res.status_code();
        let body: Value = res.json().await.unwrap_or(json!({}));
        if !(200..300).contains(&status) {
            let _ = ws.send_with_str(envelope("walkie", Some(&group), &json!({"type":"error","message": body.get("error").and_then(Value::as_str).unwrap_or("Could not join group.")})).to_string());
            return Ok(());
        }
        let connection_id = body.get("connection_id").and_then(Value::as_str).unwrap_or("").to_string();
        att.walkie.retain(|s| !(s.group_id == group && s.observe == observe));
        att.walkie.push(WalkieSub { group_id: group.clone(), connection_id: connection_id.clone(), observe });
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

    async fn walkie_forward(&self, att: &GatewayAttachment, data: &Value) -> Result<()> {
        let group = data.get("group_id").and_then(Value::as_str).unwrap_or("");
        // Demux by (group, observe): the media session (observe=false) and the
        // activity observer (observe=true) share this socket, and signals such
        // as press/call_start only mean anything on the media connection.
        let observe = data.get("observe").and_then(Value::as_bool).unwrap_or(false);
        let Some(sub) = att.walkie.iter().find(|s| s.group_id == group && s.observe == observe)
            .or_else(|| att.walkie.iter().find(|s| s.group_id == group)) else { return Ok(()) };
        let typ = data.get("type").and_then(Value::as_str).unwrap_or("");
        let path = if typ == "unsubscribe" { "/gateway-leave" } else { "/gateway-signal" };
        let _ = self.room_fetch(group, path, &json!({"token": att.id, "connection_id": sub.connection_id, "message": data})).await;
        Ok(())
    }

    async fn leave_all(&self, att: &GatewayAttachment) {
        for sub in &att.walkie {
            let _ = self.room_fetch(&sub.group_id, "/gateway-leave", &json!({"token": att.id, "connection_id": sub.connection_id, "message": {"type":"unsubscribe"}})).await;
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
                    Some("unsubscribe") => { self.walkie_forward(&att, &data).await?; let observe = data.get("observe").and_then(Value::as_bool).unwrap_or(false); att.walkie.retain(|s| !(Some(s.group_id.as_str()) == data.get("group_id").and_then(Value::as_str) && s.observe == observe)); let _ = ws.serialize_attachment(&att); }
                    _ => self.walkie_forward(&att, &data).await?,
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
                        let _ = self.room_fetch(&sub.group_id, "/gateway-signal", &json!({"token": att.id, "connection_id": sub.connection_id, "message": {"type":"heartbeat"}})).await;
                    }
                }
            }
        }
        Ok(())
    }

    async fn websocket_close(&self, ws: WebSocket, _code: usize, _reason: String, _was_clean: bool) -> Result<()> {
        if let Some(att) = ws.deserialize_attachment::<GatewayAttachment>()? { self.leave_all(&att).await; }
        Ok(())
    }

    async fn websocket_error(&self, ws: WebSocket, _error: Error) -> Result<()> {
        if let Some(att) = ws.deserialize_attachment::<GatewayAttachment>()? { self.leave_all(&att).await; }
        Ok(())
    }
}
