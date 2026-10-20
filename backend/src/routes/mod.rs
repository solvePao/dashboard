#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub mod auth;
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub mod device;
pub mod validation;

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use worker::*;

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use crate::common::DB_BINDING_NAME;
#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
use crate::models::error::AppError;

#[cfg(not(any(test, coverage)))]
#[cfg(not(test))]
pub fn get_db(ctx: &RouteContext<()>) -> std::result::Result<D1Database, AppError> {
    ctx.d1(DB_BINDING_NAME).map_err(|_| {
        AppError::service_unavailable(format!(
            "D1 database binding '{DB_BINDING_NAME}' is not configured."
        ))
    })
}

