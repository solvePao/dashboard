use worker::*;

use crate::common::{JWT_EXPIRY_SECS, SESSION_COOKIE_NAME};
use crate::models::auth::GoogleAuthRequest;
use crate::services::AuthService;
use crate::utils::{build_expired_session_cookie, build_session_cookie, json_error, json_ok};
use super::{get_db, validation};

/// Secure cookie attributes only in production, where the dashboard and
/// API may be on different sites; local dev stays `SameSite=Lax`.
fn cookie_secure(ctx: &RouteContext<()>) -> bool {
    crate::utils::env::is_production(&ctx.env)
}

/// Sign-in route (public): the ONLY route that accepts a Google ID token.
/// Returns the user plus the server-minted JWT the frontend must use
/// as `Bearer` on every other route.
pub async fn google_auth(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: GoogleAuthRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return json_error("Invalid JSON body.", 400),
    };

    let credential = match validation::require_credential(&body) {
        Ok(c) => c,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    match AuthService::login_with_google(&ctx.env, &db, &credential).await {
        Ok(login) => {
            // Also plant the session as an HttpOnly cookie: browsers
            // cannot send `Authorization` on a WebSocket handshake, so
            // the live devices hub authenticates from this cookie.
            // REST routes keep using the bearer token unchanged.
            let mut res = json_ok(&login)?;
            let cookie = build_session_cookie(
                SESSION_COOKIE_NAME,
                &login.token,
                JWT_EXPIRY_SECS,
                cookie_secure(&ctx),
            );
            let _ = res.headers_mut().set("Set-Cookie", &cookie);
            Ok(res)
        }
        Err(err) => err.to_response(),
    }
}

/// Expires the hub session cookie. Called best-effort on sign-out;
/// the bearer-token session itself is stateless and simply lapses.
pub async fn logout(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let mut res = json_ok(&serde_json::json!({ "ok": true }))?;
    let cookie = build_expired_session_cookie(SESSION_COOKIE_NAME, cookie_secure(&ctx));
    let _ = res.headers_mut().set("Set-Cookie", &cookie);
    Ok(res)
}

pub async fn get_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let sub = match validation::require_route_sub(ctx.param("sub").map(String::as_str)) {
        Ok(s) => s,
        Err(err) => return err.to_response(),
    };

    let header = req.headers().get("Authorization").unwrap_or(None);
    let token = match validation::require_bearer_header(header.as_deref()) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    // Caller must hold a valid session JWT (or device token) whose
    // identity matches the requested sub.
    let user = match AuthService::authenticate(&token, &ctx.env, &db).await {
        Ok(u) => u,
        Err(err) => return err.to_response(),
    };

    if let Err(err) = validation::require_sub_match(&user.google_sub, &sub) {
        return err.to_response();
    }

    match AuthService::get_user_by_sub(&db, &sub).await {
        Ok(user) => json_ok(&crate::models::auth::UserResponse { user }),
        Err(err) => err.to_response(),
    }
}

pub async fn get_me(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let header = req.headers().get("Authorization").unwrap_or(None);
    let token = match validation::require_bearer_header(header.as_deref()) {
        Ok(t) => t,
        Err(err) => return err.to_response(),
    };

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    match AuthService::authenticate(&token, &ctx.env, &db).await {
        Ok(user) => json_ok(&crate::models::auth::UserResponse { user }),
        Err(err) => err.to_response(),
    }
}
