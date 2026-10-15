use worker::D1Database;

use crate::db;
use crate::models::auth::User;
use crate::models::device::{DeviceSession, DeviceToken};
use crate::models::error::{AppError, AppResult};

/// Testable storage seam for services.
/// Production is backed by D1 (`D1Stores`); tests use in-memory fakes.
#[allow(async_fn_in_trait)]
pub trait UserStore {
    async fn store_get_user_by_sub(&self, sub: &str) -> AppResult<Option<User>>;
    async fn store_get_user_by_id(&self, id: i64) -> AppResult<Option<User>>;
    async fn store_get_user_by_device_token(&self, token: &str) -> AppResult<Option<User>>;
    async fn store_upsert_user(
        &self,
        sub: &str,
        email: &str,
        name: Option<&str>,
        picture_url: Option<&str>,
    ) -> AppResult<User>;
}

#[allow(async_fn_in_trait)]
pub trait DeviceStore {
    async fn store_upsert_device_session(
        &self,
        session_id: &str,
        short_code: &str,
    ) -> AppResult<DeviceSession>;
    async fn store_get_device_session(&self, session_id: &str) -> AppResult<Option<DeviceSession>>;
    async fn store_find_pending_session(
        &self,
        code_or_session_id: &str,
    ) -> AppResult<Option<DeviceSession>>;
    async fn store_authorize_device_session(
        &self,
        session_id: &str,
        user_id: i64,
        device_token: &str,
        device_name: &str,
    ) -> AppResult<()>;
    async fn store_get_user_devices(&self, user_id: i64) -> AppResult<Vec<DeviceToken>>;
    async fn store_revoke_device_token(&self, user_id: i64, token: &str) -> AppResult<bool>;
}

/// Production adapter over `D1Database`.
pub struct D1Stores<'a>(pub &'a D1Database);

impl UserStore for D1Stores<'_> {
    async fn store_get_user_by_sub(&self, sub: &str) -> AppResult<Option<User>> {
        db::get_user_by_sub(self.0, sub).await.map_err(|err| {
            worker::console_error!("D1 query failed: {err:?}");
            AppError::bad_gateway("Failed to load user.")
        })
    }

    async fn store_get_user_by_id(&self, id: i64) -> AppResult<Option<User>> {
        db::get_user_by_id(self.0, id)
            .await
            .map_err(|_| AppError::bad_gateway("Database error."))
    }

    async fn store_get_user_by_device_token(&self, token: &str) -> AppResult<Option<User>> {
        db::get_user_by_device_token(self.0, token)
            .await
            .map_err(|_| AppError::bad_gateway("Database error."))
    }

    async fn store_upsert_user(
        &self,
        sub: &str,
        email: &str,
        name: Option<&str>,
        picture_url: Option<&str>,
    ) -> AppResult<User> {
        db::upsert_user(self.0, sub, email, name, picture_url)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to save user."))
    }
}

impl DeviceStore for D1Stores<'_> {
    async fn store_upsert_device_session(
        &self,
        session_id: &str,
        short_code: &str,
    ) -> AppResult<DeviceSession> {
        db::upsert_device_session(self.0, session_id, short_code)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to save device session."))
    }

    async fn store_get_device_session(&self, session_id: &str) -> AppResult<Option<DeviceSession>> {
        db::get_device_session(self.0, session_id)
            .await
            .map_err(|_| AppError::bad_gateway("Database error."))
    }

    async fn store_find_pending_session(
        &self,
        code_or_session_id: &str,
    ) -> AppResult<Option<DeviceSession>> {
        db::find_pending_session(self.0, code_or_session_id)
            .await
            .map_err(|_| AppError::bad_gateway("Database error."))
    }

    async fn store_authorize_device_session(
        &self,
        session_id: &str,
        user_id: i64,
        device_token: &str,
        device_name: &str,
    ) -> AppResult<()> {
        db::authorize_device_session(self.0, session_id, user_id, device_token, device_name)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to authorize device session."))
    }

    async fn store_get_user_devices(&self, user_id: i64) -> AppResult<Vec<DeviceToken>> {
        db::get_user_devices(self.0, user_id)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to fetch devices."))
    }

    async fn store_revoke_device_token(&self, user_id: i64, token: &str) -> AppResult<bool> {
        db::revoke_device_token(self.0, user_id, token)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to revoke device."))
    }
}
