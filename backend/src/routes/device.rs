use worker::*;

use crate::models::device::{
    AuthorizeDeviceRequest, CheckSessionRequest, CreateSessionRequest, DevicesResponse,
    RevokeResponse,
};
use crate::services::{AuthService, DeviceService};
use crate::utils::{json_error, json_ok};
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

/// Proxies the mobile WebSocket upgrade to the session's PairingObject.
///
/// No auth token is required: the `session_id` is a 128-char random hex string,
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

    // The incoming request carries immutable headers, which the runtime
    // refuses to forward into a subrequest as-is ("Can't modify immutable
    // headers"). Rebuild a fresh upgrade request for the object instead.
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
    let do_req = match Request::new_with_init("https://pairing/ws", &init) {
        Ok(r) => r,
        Err(_) => return json_error("Failed to reach pairing session.", 502),
    };

    stub.fetch_with_request(do_req).await
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
    let do_req = match Request::new_with_init("https://device/ws", &init) {
        Ok(r) => r,
        Err(_) => return json_error("Failed to reach device session.", 502),
    };

    stub.fetch_with_request(do_req).await
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

    match DeviceService::revoke_device(&db, &ctx.env, user.id, &token_to_revoke).await {
        Ok(revoked) => json_ok(&RevokeResponse { revoked }),
        Err(err) => err.to_response(),
    }
}
