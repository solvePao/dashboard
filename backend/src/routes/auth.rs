use worker::*;

use crate::models::auth::{GoogleAuthRequest, UserResponse};
use crate::services::AuthService;
use crate::utils::{json_error, json_ok};
use super::{get_db, validation};

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
        Ok(user) => json_ok(&UserResponse { user }),
        Err(err) => err.to_response(),
    }
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

    // Caller must authenticate with Google ID token matching the requested sub
    let profile = match AuthService::verify_google_id_token(&ctx.env, &token).await {
        Ok(p) => p,
        Err(err) => return err.to_response(),
    };

    if let Err(err) = validation::require_sub_match(&profile.sub, &sub) {
        return err.to_response();
    }

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    match AuthService::get_user_by_sub(&db, &sub).await {
        Ok(user) => json_ok(&UserResponse { user }),
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
        Ok(user) => json_ok(&UserResponse { user }),
        Err(err) => err.to_response(),
    }
}
