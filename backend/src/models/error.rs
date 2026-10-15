use serde::{Deserialize, Serialize};
use worker::*;

use crate::utils::json_error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub message: String,
    pub status: u16,
}

impl AppError {
    pub fn new(message: impl Into<String>, status: u16) -> Self {
        Self {
            message: message.into(),
            status,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(message, 400)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(message, 401)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(message, 403)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(message, 404)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(message, 500)
    }

    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self::new(message, 502)
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(message, 503)
    }

    pub fn to_response(&self) -> Result<Response> {
        json_error(&self.message, self.status)
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;

