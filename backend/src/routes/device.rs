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

    match DeviceService::create_session(&db, &session_id).await {
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
        Err(err) => return err.to_response(),
    };

    match DeviceService::authorize_device(
        &db,
        user.id,
        &code_or_session,
        body.device_name.as_deref(),
    )
    .await
    {
        Ok(res) => json_ok(&res),
        Err(err) => err.to_response(),
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

    let db = match get_db(&ctx) {
        Ok(db) => db,
        Err(err) => return err.to_response(),
    };

    match DeviceService::check_session(&db, &session_id).await {
        Ok(res) => json_ok(&res),
        Err(err) => err.to_response(),
    }
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

    match DeviceService::revoke_device(&db, user.id, &token_to_revoke).await {
        Ok(revoked) => json_ok(&RevokeResponse { revoked }),
        Err(err) => err.to_response(),
    }
}
