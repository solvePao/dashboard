pub mod auth;
pub mod device;
pub mod health;
pub mod validation;

use worker::*;

use crate::common::DB_BINDING_NAME;
use crate::models::error::AppError;

pub fn get_db(ctx: &RouteContext<()>) -> std::result::Result<D1Database, AppError> {
    ctx.d1(DB_BINDING_NAME).map_err(|_| {
        AppError::service_unavailable(format!(
            "D1 database binding '{DB_BINDING_NAME}' is not configured."
        ))
    })
}

