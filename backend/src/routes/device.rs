use worker::*;

use crate::models::device::{
    AuthorizeDeviceRequest, CheckSessionRequest, CreateSessionRequest, DevicesResponse,
    RevokeResponse, SaveWidgetsRequest, WidgetsResponse,
};
use crate::services::{AuthService, DeviceService};
use crate::utils::{
    allowed_origins, is_origin_allowed, json_error, json_ok, parse_cookie_header, request_origin,
};
use super::{get_db, validation};

pub async fn create_session(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: CreateSessionRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return json_error("Invalid JSON body.", 400),
    };

    let session_id = match validation::require_session_id(&body) {
        Ok(id) => id,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    match DeviceService::create_session(&db, &ctx.env, &session_id).await {
        Ok(res) => json_ok(&res),
        Err(err) => err.to_response(),
    }
}

pub async fn authorize_device(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let header = req.headers().get("Authorization").unwrap_or(None);
    let auth_token = match validation::require_bearer_header(header.as_deref()) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    let user = match AuthService::authenticate(&auth_token, &ctx.env, &db).await {
        Ok(u) => u,
        Err(err) => return err.to_response(),
    };

    let body: AuthorizeDeviceRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return json_error("Invalid JSON body.", 400),
    };

    let code_or_session = match validation::resolve_code_or_session(&body) {
        Ok(c) => c,
        Err(err) => {
            Delay::from(std::time::Duration::from_secs(crate::common::FAILED_AUTHORIZE_DELAY_SECS)).await;
            return err.to_response();
        }
    };

    // Failed authorization attempts are delayed by 5 seconds to deter
    // brute-force guessing of pairing codes without D1-backed rate limiting.
    match DeviceService::authorize_device(
        &db,
        &ctx.env,
        &user,
        &code_or_session,
        body.device_name.as_deref(),
    )
    .await
    {
        Ok(res) => json_ok(&res),
        Err(err) => {
            Delay::from(std::time::Duration::from_secs(crate::common::FAILED_AUTHORIZE_DELAY_SECS)).await;
            err.to_response()
        }
    }
}

pub async fn check_session(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: CheckSessionRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return json_error("Invalid JSON body.", 400),
    };

    let session_id = match validation::require_check_session_id(&body) {
        Ok(id) => id,
        Err(err) => return err.to_response(),
    };

    match DeviceService::check_session(&ctx.env, &session_id).await {
        Ok(res) => json_ok(&res),
        Err(err) => err.to_response(),
    }
}

/// Extracts WebSocket upgrade headers from an incoming request, builds a fresh
/// subrequest targeting the specified internal Durable Object route, and dispatches it.
/// Rebuilding the request is necessary because the incoming request carries immutable
/// headers which Cloudflare Workers refuses to forward directly into a subrequest.
async fn forward_websocket_to_stub(
    req: &Request,
    stub: &Stub,
    target_url: &str,
    error_msg: &'static str,
) -> Result<Response> {
    let headers = Headers::new();
    for name in [
        "upgrade",
        "connection",
        "sec-websocket-key",
        "sec-websocket-version",
        "sec-websocket-protocol",
        "sec-websocket-extensions",
    ] {
        if let Ok(Some(value)) = req.headers().get(name) {
            let _ = headers.set(name, &value);
        }
    }
    let mut init = RequestInit::new();
    init.method = Method::Get;
    init.headers = headers;
    let do_req = match Request::new_with_init(target_url, &init) {
        Ok(r) => r,
        Err(_) => return json_error(error_msg, 502),
    };

    stub.fetch_with_request(do_req).await
}

/// Proxies the mobile WebSocket upgrade to the session's PairingObject.
///
/// No auth token is required: the `session_id` is a 32-char random hex string,
/// making it computationally unguessable. Accepted trade-off: anyone who can observe
/// the session_id (e.g. via QR interception) can connect and receive the push.
/// The PairingObject's Wipe alarm (2 min) closes the replay window once the result
/// is delivered or the session expires.
pub async fn pairing_socket(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let raw = ctx.param("session_id").map(String::as_str).unwrap_or("");
    let session_id = match crate::services::validation::validate_session_id(raw) {
        Ok(id) => id,
        Err(err) => return err.to_response(),
    };

    let client = match crate::services::DoPairingClient::from_env(&ctx.env) {
        Ok(c) => c,
        Err(err) => return err.to_response(),
    };

    let stub = match client.namespace().get_by_name(&session_id) {
        Ok(s) => s,
        Err(_) => return json_error("Failed to reach pairing session.", 502),
    };

    forward_websocket_to_stub(&req, &stub, "https://pairing/ws", "Failed to reach pairing session.").await
}

/// Proxies the mobile device WebSocket upgrade to the device's DeviceObject.
pub async fn device_socket(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let token = match validation::require_route_token(ctx.param("token").map(String::as_str)) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    match crate::db::users::get_user_by_device_token(&db, &token).await {
        Ok(Some(_)) => {}
        Ok(None) => return json_error("Device token not found or expired.", 401),
        Err(_) => return json_error("Database error.", 500),
    }

    let namespace = match ctx.env.durable_object(crate::common::DEVICE_BINDING_NAME) {
        Ok(ns) => ns,
        Err(_) => return json_error("Failed to reach device session.", 502),
    };

    let stub = match namespace.get_by_name(&token) {
        Ok(s) => s,
        Err(_) => return json_error("Failed to reach device session.", 502),
    };

    forward_websocket_to_stub(&req, &stub, "https://device/ws", "Failed to reach device session.").await
}

/// Proxies the web dashboard WebSocket upgrade to the user's
/// `DevicesHubObject`. Browsers cannot send `Authorization` on a
/// WebSocket handshake, so the hub authenticates from the HttpOnly
/// session cookie planted at login. Because cookies are ambient, the
/// `Origin` must match a configured frontend origin (CSWSH guard).
/// The hub is addressed by the server-derived user id, never by any
/// client-supplied identifier.
pub async fn devices_live(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let allowed = allowed_origins(&ctx.env);
    let origin = request_origin(&req);
    if !is_origin_allowed(origin.as_deref(), &allowed) {
        return json_error("Origin not allowed.", 403);
    }

    let cookie_header = req.headers().get("Cookie").unwrap_or(None);
    let token = match parse_cookie_header(cookie_header.as_deref(), crate::common::SESSION_COOKIE_NAME) {
        Some(t) => t,
        None => return json_error("Missing session cookie.", 401),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    let user = match AuthService::authenticate(&token, &ctx.env, &db).await {
        Ok(u) => u,
        Err(err) => return err.to_response(),
    };

    let namespace = match ctx.env.durable_object(crate::common::HUB_BINDING_NAME) {
        Ok(ns) => ns,
        Err(_) => return json_error("Failed to reach devices hub.", 502),
    };

    let stub = match namespace.get_by_name(&user.id.to_string()) {
        Ok(s) => s,
        Err(_) => return json_error("Failed to reach devices hub.", 502),
    };

    forward_websocket_to_stub(&req, &stub, "https://hub/ws", "Failed to reach devices hub.").await
}

pub async fn get_devices(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let header = req.headers().get("Authorization").unwrap_or(None);
    let token = match validation::require_bearer_header(header.as_deref()) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    let user = match AuthService::authenticate(&token, &ctx.env, &db).await {
        Ok(u) => u,
        Err(err) => return err.to_response(),
    };

    match DeviceService::get_user_devices(&db, user.id).await {
        Ok(devices) => json_ok(&DevicesResponse { devices }),
        Err(err) => err.to_response(),
    }
}

pub async fn revoke_device(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let token_to_revoke =
        match validation::require_route_token(ctx.param("token").map(String::as_str)) {
            Ok(t) => t,
            Err(err) => return err.to_response(),
        };

    let header = req.headers().get("Authorization").unwrap_or(None);
    let auth_token = match validation::require_bearer_header(header.as_deref()) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    let user = match AuthService::authenticate(&auth_token, &ctx.env, &db).await {
        Ok(u) => u,
        Err(err) => return err.to_response(),
    };

    // A caller deleting its own token (mobile Disconnect) is a
    // self-disconnect; any other own-device deletion is a revocation.
    let kind = validation::resolve_lifecycle_kind(&auth_token, &token_to_revoke);

    match DeviceService::revoke_device(&db, &ctx.env, user.id, &token_to_revoke, kind).await {
        Ok(revoked) => json_ok(&RevokeResponse { revoked }),
        Err(err) => err.to_response(),
    }
}

/// Reads one device's widget set for the web dashboard. The caller must
/// own the token; other users' tokens report 404.
pub async fn get_device_widgets(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let token = match validation::require_route_token(ctx.param("token").map(String::as_str)) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let header = req.headers().get("Authorization").unwrap_or(None);
    let auth_token = match validation::require_bearer_header(header.as_deref()) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    let user = match AuthService::authenticate(&auth_token, &ctx.env, &db).await {
        Ok(u) => u,
        Err(err) => return err.to_response(),
    };

    match DeviceService::get_device_widgets(&db, user.id, &token).await {
        Ok(widgets) => json_ok(&WidgetsResponse { widgets }),
        Err(err) => err.to_response(),
    }
}

/// Replaces one device's widget set from the web dashboard. An empty
/// list is valid and clears every widget (the "None" choice).
pub async fn save_device_widgets(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let token = match validation::require_route_token(ctx.param("token").map(String::as_str)) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let header = req.headers().get("Authorization").unwrap_or(None);
    let auth_token = match validation::require_bearer_header(header.as_deref()) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    let user = match AuthService::authenticate(&auth_token, &ctx.env, &db).await {
        Ok(u) => u,
        Err(err) => return err.to_response(),
    };

    let body: SaveWidgetsRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return json_error("Invalid JSON body.", 400),
    };

    let widgets = match crate::services::validation::normalize_widgets(body.widgets.as_deref()) {
        Ok(w) => w,
        Err(err) => return err.to_response(),
    };

    match DeviceService::save_device_widgets(&db, &ctx.env, user.id, &token, widgets).await {
        Ok(widgets) => json_ok(&WidgetsResponse { widgets }),
        Err(err) => err.to_response(),
    }
}

/// Reads the caller's own widget set using the device token as the
/// Bearer [REDACTED] (mobile clients). Unknown/expired tokens are 401.
pub async fn get_own_widgets(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let header = req.headers().get("Authorization").unwrap_or(None);
    let token = match validation::require_bearer_header(header.as_deref()) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    match DeviceService::get_own_widgets(&db, &token).await {
        Ok(widgets) => json_ok(&WidgetsResponse { widgets }),
        Err(err) => err.to_response(),
    }
}
