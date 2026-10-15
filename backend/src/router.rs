pub mod common;
pub mod db;
pub mod models;
pub mod routes;
pub mod services;
pub mod utils;

use utils::{allowed_origins, apply_cors, cors_preflight, request_origin};
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let allowed = allowed_origins(&env);
    let origin = request_origin(&req);

    if req.method() == Method::Options {
        return cors_preflight(origin.as_deref(), &allowed);
    }

    let res = Router::new()
        // Health check
        .get("/health", routes::health::health)
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
        .get_async("/api/devices", |req, ctx| async move {
            routes::device::get_devices(req, ctx).await
        })
        .delete_async("/api/devices/:token", |req, ctx| async move {
            routes::device::revoke_device(req, ctx).await
        })
        .run(req, env)
        .await?;

    apply_cors(origin.as_deref(), &allowed, res)
}
