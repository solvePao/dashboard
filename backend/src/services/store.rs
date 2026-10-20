#[cfg(not(any(test, coverage)))]
use worker::D1Database;

use crate::common::DeviceLifecycleKind;
#[cfg(not(any(test, coverage)))]
use crate::db;
use crate::models::auth::User;
use crate::models::device::{CheckSessionResponse, DeviceToken, DeviceWidget, PairingCode, PairingState};
#[cfg(not(any(test, coverage)))]
use crate::models::error::AppError;
use crate::models::error::AppResult;

/// Testable storage seams for services.
/// Production D1 is `D1Stores`; the pairing referee is a Durable
/// Object behind `PairingClient` (see `services::pairing`).
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

/// D1 side of pairing: code index + long-lived device tokens only.
/// Pairing state never lives here.
#[allow(async_fn_in_trait)]
pub trait PairingStore {
    async fn store_upsert_pairing_code(
        &self,
        session_id: &str,
        short_code: &str,
    ) -> AppResult<PairingCode>;
    async fn store_find_session_by_code(&self, code: &str) -> AppResult<Option<PairingCode>>;
    async fn store_get_pairing_code(&self, session_id: &str) -> AppResult<Option<PairingCode>>;
    async fn store_insert_device_token(
        &self,
        user_id: i64,
        device_token: &str,
        device_name: &str,
    ) -> AppResult<()>;
    async fn store_delete_device_token(&self, device_token: &str) -> AppResult<()>;
    async fn store_get_user_devices(&self, user_id: i64) -> AppResult<Vec<DeviceToken>>;
    async fn store_revoke_device_token(&self, user_id: i64, token: &str) -> AppResult<bool>;
}

/// Push side of revocation: tells a live device its token just died.
/// Fire-and-forget — errors are intentionally swallowed at every call site.
#[allow(async_fn_in_trait)]
pub trait DeviceNotifier {
    async fn notify_revoked(&self, device_token: &str);
    /// Tells a live device its widget set changed. Unlike revocation the
    /// socket stays open; the device re-fetches and re-renders.
    async fn notify_widgets_updated(&self, device_token: &str);
}

/// Used when the notifier binding is unavailable: revocation still commits.
pub struct NoNotify;

impl DeviceNotifier for NoNotify {
    async fn notify_revoked(&self, _device_token: &str) {}
    async fn notify_widgets_updated(&self, _device_token: &str) {}
}

/// D1 side of per-device widgets: config rows only. Widget semantics
/// (known keys, config shapes) live on the frontends' registries.
#[allow(async_fn_in_trait)]
pub trait WidgetStore {
    async fn store_device_token_owner(&self, token: &str) -> AppResult<Option<i64>>;
    async fn store_get_device_widgets(&self, token: &str) -> AppResult<Vec<DeviceWidget>>;
    async fn store_replace_device_widgets(
        &self,
        token: &str,
        widgets: &[DeviceWidget],
    ) -> AppResult<()>;
}

/// Live-dashboard side of the lifecycle: tells the user's hub object
/// that a device was authorized, revoked, or disconnected. Like
/// `DeviceNotifier`, this is fire-and-forget at every call site.
#[allow(async_fn_in_trait)]
pub trait HubNotifier {
    async fn notify_hub(&self, user_id: i64, kind: DeviceLifecycleKind, device_token: Option<&str>);
}

impl HubNotifier for NoNotify {
    async fn notify_hub(&self, _user_id: i64, _kind: DeviceLifecycleKind, _device_token: Option<&str>) {}
}

/// Durable Object side of pairing: the state referee.
#[allow(async_fn_in_trait)]
pub trait PairingClient {
    async fn client_init(&self, session_id: &str, expires_at: &str) -> AppResult<PairingState>;
    async fn client_status(&self, session_id: &str) -> AppResult<Option<CheckSessionResponse>>;
    /// `Ok(false)` means the object refused (already authorized).
    async fn client_commit(
        &self,
        session_id: &str,
        device_token: &str,
        user: &User,
    ) -> AppResult<bool>;
}

/// Production adapter over `D1Database`.
#[cfg(not(any(test, coverage)))]
pub struct D1Stores<'a>(pub &'a D1Database);

#[cfg(not(any(test, coverage)))]
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

#[cfg(not(any(test, coverage)))]
impl PairingStore for D1Stores<'_> {
    async fn store_upsert_pairing_code(
        &self,
        session_id: &str,
        short_code: &str,
    ) -> AppResult<PairingCode> {
        db::upsert_pairing_code(self.0, session_id, short_code)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to save pairing code."))
    }

    async fn store_find_session_by_code(&self, code: &str) -> AppResult<Option<PairingCode>> {
        db::find_session_by_code(self.0, code)
            .await
            .map_err(|_| AppError::bad_gateway("Database error."))
    }

    async fn store_get_pairing_code(&self, session_id: &str) -> AppResult<Option<PairingCode>> {
        db::get_pairing_code(self.0, session_id)
            .await
            .map_err(|_| AppError::bad_gateway("Database error."))
    }

    async fn store_insert_device_token(
        &self,
        user_id: i64,
        device_token: &str,
        device_name: &str,
    ) -> AppResult<()> {
        db::insert_device_token(self.0, user_id, device_token, device_name)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to authorize device session."))
    }

    async fn store_delete_device_token(&self, device_token: &str) -> AppResult<()> {
        db::delete_device_token(self.0, device_token)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to clean up device token."))
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

#[cfg(not(any(test, coverage)))]
impl WidgetStore for D1Stores<'_> {
    async fn store_device_token_owner(&self, token: &str) -> AppResult<Option<i64>> {
        db::get_device_token_owner(self.0, token)
            .await
            .map_err(|_| AppError::bad_gateway("Database error."))
    }

    async fn store_get_device_widgets(&self, token: &str) -> AppResult<Vec<DeviceWidget>> {
        db::get_device_widgets(self.0, token)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to fetch widgets."))
    }

    async fn store_replace_device_widgets(
        &self,
        token: &str,
        widgets: &[DeviceWidget],
    ) -> AppResult<()> {
        db::replace_device_widgets(self.0, token, widgets)
            .await
            .map_err(|_| AppError::bad_gateway("Failed to save widgets."))
    }
}
