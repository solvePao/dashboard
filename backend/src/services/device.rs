use worker::*;

use super::store::{D1Stores, DeviceStore, UserStore};
use super::validation as v;
use crate::models::device::{
    AuthorizeDeviceResponse, CheckSessionResponse, CreateSessionResponse, DeviceToken,
};
use crate::models::error::{AppError, AppResult};
use crate::utils;

pub struct DeviceService;

impl DeviceService {
    /// Creates a device pairing session with QR code SVG and short code.
    pub async fn create_session(
        db: &D1Database,
        session_id: &str,
    ) -> AppResult<CreateSessionResponse> {
        Self::create_session_with(&D1Stores(db), session_id, utils::generate_qr_svg).await
    }

    pub async fn create_session_with<S, F>(
        store: &S,
        session_id: &str,
        generate_qr: F,
    ) -> AppResult<CreateSessionResponse>
    where
        S: DeviceStore + Sync,
        F: FnOnce(&str) -> std::result::Result<String, String>,
    {
        let trimmed_id = v::validate_session_id(session_id)?;
        let short_code = v::derive_short_code(&trimmed_id);
        let qr_svg = generate_qr(&trimmed_id)
            .map_err(|err| AppError::internal(format!("Failed to generate QR: {err}")))?;

        let session = store
            .store_upsert_device_session(&trimmed_id, &short_code)
            .await?;

        Ok(CreateSessionResponse {
            session_id: session.session_id,
            short_code: v::present_short_code(&session.short_code),
            qr_svg,
            expires_at: session.expires_at,
        })
    }

    /// Authorizes a pending session by pairing it with the authenticated user and creating a device token.
    pub async fn authorize_device(
        db: &D1Database,
        user_id: i64,
        code_or_session: &str,
        device_name: Option<&str>,
    ) -> AppResult<AuthorizeDeviceResponse> {
        Self::authorize_device_with(
            &D1Stores(db),
            user_id,
            code_or_session,
            device_name,
            utils::generate_secure_token,
        )
        .await
    }

    pub async fn authorize_device_with<S, F>(
        store: &S,
        user_id: i64,
        code_or_session: &str,
        device_name: Option<&str>,
        generate_token: F,
    ) -> AppResult<AuthorizeDeviceResponse>
    where
        S: DeviceStore + Sync,
        F: FnOnce() -> std::result::Result<String, String>,
    {
        let trimmed_code = v::normalize_code_or_session(code_or_session)?;

        let pending = store.store_find_pending_session(&trimmed_code).await?;
        let session = match pending {
            Some(s) => s,
            None => {
                // Distinguish already-authorized (409) from unknown/expired
                // (404) so a second scanner gets an honest conflict rather
                // than a misleading not-found.
                if let Ok(Some(existing)) =
                    store.store_find_session_for_authorize(&trimmed_code).await
                {
                    if existing.status == "authorized" {
                        return Err(AppError::new(
                            "Session was already authorized.",
                            409,
                        ));
                    }
                }
                return Err(AppError::not_found("Pending session not found or expired."));
            }
        };

        let device_token = generate_token().map_err(AppError::internal)?;

        let name = v::normalize_device_name(device_name);

        let authorized = store
            .store_authorize_device_session(&session.session_id, user_id, &device_token, &name)
            .await?;
        if !authorized {
            return Err(AppError::new(
                "Session was already authorized or expired.",
                409,
            ));
        }

        Ok(AuthorizeDeviceResponse {
            status: "authorized".to_string(),
            session_id: session.session_id,
            device_name: name,
            message: "Device successfully authorized.".to_string(),
        })
    }

    /// Checks the status of a device session (polling).
    /// If authorized, returns the device token and user. If expired, returns status 'expired'.
    pub async fn check_session(db: &D1Database, session_id: &str) -> AppResult<CheckSessionResponse> {
        Self::check_session_with(&D1Stores(db), session_id).await
    }

    pub async fn check_session_with<S>(store: &S, session_id: &str) -> AppResult<CheckSessionResponse>
    where
        S: DeviceStore + UserStore + Sync,
    {
        let trimmed_id = v::normalize_session_lookup(session_id)?;

        let session = store
            .store_get_device_session(&trimmed_id)
            .await?
            .ok_or_else(|| AppError::not_found("Session not found."))?;

        if session.status == "authorized" {
            if let (Some(token), Some(user_id)) = (session.device_token, session.user_id) {
                let user = store.store_get_user_by_id(user_id).await.unwrap_or(None);
                return Ok(CheckSessionResponse {
                    status: "authorized".to_string(),
                    token: Some(token),
                    user,
                });
            }
        }

        Ok(CheckSessionResponse {
            status: session.status,
            token: None,
            user: None,
        })
    }

    /// Lists active device tokens for the user.
    pub async fn get_user_devices(db: &D1Database, user_id: i64) -> AppResult<Vec<DeviceToken>> {
        Self::get_user_devices_with(&D1Stores(db), user_id).await
    }

    pub async fn get_user_devices_with<S>(
        store: &S,
        user_id: i64,
    ) -> AppResult<Vec<DeviceToken>>
    where
        S: DeviceStore + Sync,
    {
        store.store_get_user_devices(user_id).await
    }

    /// Revokes a device token for the user.
    pub async fn revoke_device(db: &D1Database, user_id: i64, token: &str) -> AppResult<bool> {
        Self::revoke_device_with(&D1Stores(db), user_id, token).await
    }

    pub async fn revoke_device_with<S>(store: &S, user_id: i64, token: &str) -> AppResult<bool>
    where
        S: DeviceStore + Sync,
    {
        store.store_revoke_device_token(user_id, token).await
    }
}
