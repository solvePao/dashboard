pub mod common;
pub mod db;
pub mod models;
pub mod objects;
pub mod routes;
pub mod services;
pub mod utils;

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub use objects::{DeviceObject, DevicesHubObject, PairingObject};

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use utils::{
    allowed_origins, apply_cors, cors_preflight, is_websocket_upgrade, request_origin,
};
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use worker::*;

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let allowed = allowed_origins(&env);
    let origin = request_origin(&req);

    if req.method() == Method::Options {
        return cors_preflight(origin.as_deref(), &allowed);
    }

    let res = Router::new()
        // Auth & User routes
        .post_async("/api/auth/google", |req, ctx| async move {
            routes::auth::google_auth(req, ctx).await
        })
        .get_async("/api/users/:sub", |req, ctx| async move {
            routes::auth::get_user(req, ctx).await
        })
        .get_async("/api/auth/me", |req, ctx| async move {
            routes::auth::get_me(req, ctx).await
        })
        .post_async("/api/auth/logout", |req, ctx| async move {
            routes::auth::logout(req, ctx).await
        })
        // Device pairing & QR routes
        .post_async("/api/auth/device/session", |req, ctx| async move {
            routes::device::create_session(req, ctx).await
        })
        .post_async("/api/auth/device/authorize", |req, ctx| async move {
            routes::device::authorize_device(req, ctx).await
        })
        .post_async("/api/auth/device/check", |req, ctx| async move {
            routes::device::check_session(req, ctx).await
        })
        .get_async("/api/auth/device/ws/:session_id", |req, ctx| async move {
            routes::device::pairing_socket(req, ctx).await
        })
        .get_async("/api/devices/live", |req, ctx| async move {
            routes::device::devices_live(req, ctx).await
        })
        .get_async("/api/devices", |req, ctx| async move {
            routes::device::get_devices(req, ctx).await
        })
        .get_async("/api/devices/ws/:token", |req, ctx| async move {
            routes::device::device_socket(req, ctx).await
        })
        .delete_async("/api/devices/:token", |req, ctx| async move {
            routes::device::revoke_device(req, ctx).await
        })
        .get_async("/api/devices/:token/widgets", |req, ctx| async move {
            routes::device::get_device_widgets(req, ctx).await
        })
        .put_async("/api/devices/:token/widgets", |req, ctx| async move {
            routes::device::save_device_widgets(req, ctx).await
        })
        .get_async("/api/device-widgets", |req, ctx| async move {
            routes::device::get_own_widgets(req, ctx).await
        })
        .run(req, env)
        .await?;

    // Upgrades pass through untouched (see `is_websocket_upgrade`).
    if is_websocket_upgrade(res.status_code()) {
        return Ok(res);
    }
    apply_cors(origin.as_deref(), &allowed, res)
}
