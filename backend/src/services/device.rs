use worker::*;

use super::pairing::{DoDeviceNotifier, DoHubNotifier, DoPairingClient};
use super::store::{D1Stores, DeviceNotifier, HubNotifier, NoNotify, PairingClient, PairingStore};
use super::validation as v;
use crate::common::DeviceLifecycleKind;
use crate::models::auth::User;
use crate::models::device::{
    AuthorizeDeviceResponse, CheckSessionResponse, CreateSessionResponse, DeviceToken,
};
use crate::models::error::{AppError, AppResult};
use crate::utils;

pub struct DeviceService;

impl DeviceService {
    /// Creates a pairing: D1 code index + Durable Object pending state.
    pub async fn create_session(
        db: &D1Database,
        env: &Env,
        session_id: &str,
    ) -> AppResult<CreateSessionResponse> {
        let client = DoPairingClient::from_env(env)?;
        Self::create_session_with(&D1Stores(db), &client, session_id, utils::generate_qr_svg)
            .await
    }

    pub async fn create_session_with<S, C, F>(
        store: &S,
        client: &C,
        session_id: &str,
        generate_qr: F,
    ) -> AppResult<CreateSessionResponse>
    where
        S: PairingStore + Sync,
        C: PairingClient + Sync,
        F: FnOnce(&str) -> std::result::Result<String, String>,
    {
        let trimmed_id = v::validate_session_id(session_id)?;
        let short_code = v::derive_short_code(&trimmed_id);
        let qr_svg = generate_qr(&trimmed_id)
            .map_err(|err| AppError::internal(format!("Failed to generate QR: {err}")))?;

        // A finished pairing is never reopened by re-creating it.
        if let Some(status) = client.client_status(&trimmed_id).await? {
            if status.status == "authorized" {
                let row = store
                    .store_get_pairing_code(&trimmed_id)
                    .await?
                    .ok_or_else(|| AppError::not_found("Session not found."))?;
                return Ok(CreateSessionResponse {
                    session_id: trimmed_id,
                    short_code: v::present_short_code(&row.short_code),
                    qr_svg,
                    expires_at: row.expires_at,
                });
            }
        }

        let row = store
            .store_upsert_pairing_code(&trimmed_id, &short_code)
            .await?;
        let pairing = client.client_init(&trimmed_id, &row.expires_at).await?;

        Ok(CreateSessionResponse {
            session_id: trimmed_id,
            short_code: v::present_short_code(&short_code),
            qr_svg,
            expires_at: pairing.expires_at,
        })
    }

    /// Authorizes a pending pairing for the authenticated user.
    /// The object referees the transition; D1 only persists the token.
    pub async fn authorize_device(
        db: &D1Database,
        env: &Env,
        user: &User,
        code_or_session: &str,
        device_name: Option<&str>,
    ) -> AppResult<AuthorizeDeviceResponse> {
        let client = DoPairingClient::from_env(env)?;
        match DoHubNotifier::from_env(env) {
            Ok(hub) => {
                Self::authorize_device_and_notify_with(
                    &D1Stores(db),
                    &client,
                    &hub,
                    user,
                    code_or_session,
                    device_name,
                    utils::generate_secure_token,
                )
                .await
            }
            Err(_) => {
                Self::authorize_device_and_notify_with(
                    &D1Stores(db),
                    &client,
                    &NoNotify,
                    user,
                    code_or_session,
                    device_name,
                    utils::generate_secure_token,
                )
                .await
            }
        }
    }

    pub async fn authorize_device_and_notify_with<S, C, H, F>(
        store: &S,
        client: &C,
        hub_notifier: &H,
        user: &User,
        code_or_session: &str,
        device_name: Option<&str>,
        gen_token: F,
    ) -> AppResult<AuthorizeDeviceResponse>
    where
        S: PairingStore + Sync,
        C: PairingClient + Sync,
        H: HubNotifier + Sync,
        F: FnOnce() -> std::result::Result<String, String>,
    {
        let result = Self::authorize_device_with(
            store,
            client,
            user,
            code_or_session,
            device_name,
            gen_token,
        )
        .await;
        if result.is_ok() {
            hub_notifier
                .notify_hub(user.id, DeviceLifecycleKind::Authorized, None)
                .await;
        }
        result
    }

    pub async fn authorize_device_with<S, C, F>(
        store: &S,
        client: &C,
        user: &User,
        code_or_session: &str,
        device_name: Option<&str>,
        generate_token: F,
    ) -> AppResult<AuthorizeDeviceResponse>
    where
        S: PairingStore + Sync,
        C: PairingClient + Sync,
        F: FnOnce() -> std::result::Result<String, String>,
    {
        let trimmed_code = v::normalize_code_or_session(code_or_session)?;

        let session_id = if utils::is_valid_session_id_hex(&trimmed_code) {
            trimmed_code.clone()
        } else {
            match store.store_find_session_by_code(&trimmed_code).await? {
                Some(row) => row.session_id,
                None => {
                    return Err(AppError::not_found(
                        "Pending session not found or expired.",
                    ))
                }
            }
        };

        // Optimisation only — not the race guard. A concurrent authorizer can pass
        // this check and still lose at client_commit below. The Durable Object is the
        // serialization point; this pre-check just avoids a needless token mint.
        let status = client
            .client_status(&session_id)
            .await?
            .ok_or_else(|| AppError::not_found("Pending session not found or expired."))?;
        match status.status.as_str() {
            "authorized" => {
                return Err(AppError::new("Session was already authorized.", 409));
            }
            "pending" => {}
            _ => {
                return Err(AppError::not_found(
                    "Pending session not found or expired.",
                ));
            }
        }

        let device_token = generate_token().map_err(AppError::internal)?;
        let name = v::normalize_device_name(device_name);

        store
            .store_insert_device_token(user.id, &device_token, &name)
            .await?;

        // The Durable Object serializes concurrent commits. Ok(false) means another
        // authorizer already committed (409 race); compensate by deleting the token.
        match client.client_commit(&session_id, &device_token, user).await {
            Ok(true) => Ok(AuthorizeDeviceResponse {
                status: "authorized".to_string(),
                session_id,
                device_name: name,
                message: "Device successfully authorized.".to_string(),
            }),
            Ok(false) => {
                let _ = store.store_delete_device_token(&device_token).await;
                Err(AppError::new("Session was already authorized.", 409))
            }
            Err(err) => {
                let _ = store.store_delete_device_token(&device_token).await;
                Err(err)
            }
        }
    }

    /// Fallback status read; the Durable Object is the source.
    pub async fn check_session(env: &Env, session_id: &str) -> AppResult<CheckSessionResponse> {
        let client = DoPairingClient::from_env(env)?;
        Self::check_session_with(&client, session_id).await
    }

    pub async fn check_session_with<C>(client: &C, session_id: &str) -> AppResult<CheckSessionResponse>
    where
        C: PairingClient + Sync,
    {
        let trimmed_id = v::normalize_session_lookup(session_id)?;
        client
            .client_status(&trimmed_id)
            .await?
            .ok_or_else(|| AppError::not_found("Session not found."))
    }

    /// Lists active device tokens for the user.
    pub async fn get_user_devices(db: &D1Database, user_id: i64) -> AppResult<Vec<DeviceToken>> {
        Self::get_user_devices_with(&D1Stores(db), user_id).await
    }

    pub async fn get_user_devices_with<S>(store: &S, user_id: i64) -> AppResult<Vec<DeviceToken>>
    where
        S: PairingStore + Sync,
    {
        store.store_get_user_devices(user_id).await
    }

    /// Revokes a device token for the user and pushes the revocation to
    /// any live device socket. The push is best-effort: a dead object
    /// never fails the revocation itself.
    /// `kind` is `DeviceLifecycleKind::Revoked` for web-initiated revocation and
    /// `DeviceLifecycleKind::Disconnected` when the device deletes itself; both share this
    /// one delete-and-notify path (D1 first, then the object pokes).
    pub async fn revoke_device(
        db: &D1Database,
        env: &Env,
        user_id: i64,
        token: &str,
        kind: DeviceLifecycleKind,
    ) -> AppResult<bool> {
        let dev = DoDeviceNotifier::from_env(env);
        let hub = DoHubNotifier::from_env(env);
        match (dev, hub) {
            (Ok(d), Ok(h)) => {
                Self::revoke_device_and_notify_with(&D1Stores(db), &d, &h, user_id, token, kind).await
            }
            (Ok(d), Err(_)) => {
                Self::revoke_device_and_notify_with(&D1Stores(db), &d, &NoNotify, user_id, token, kind).await
            }
            (Err(_), Ok(h)) => {
                Self::revoke_device_and_notify_with(&D1Stores(db), &NoNotify, &h, user_id, token, kind).await
            }
            (Err(_), Err(_)) => {
                Self::revoke_device_and_notify_with(&D1Stores(db), &NoNotify, &NoNotify, user_id, token, kind).await
            }
        }
    }

    pub async fn revoke_device_and_notify_with<S, N, H>(
        store: &S,
        device_notifier: &N,
        hub_notifier: &H,
        user_id: i64,
        token: &str,
        kind: DeviceLifecycleKind,
    ) -> AppResult<bool>
    where
        S: PairingStore + Sync,
        N: DeviceNotifier + Sync,
        H: HubNotifier + Sync,
    {
        let revoked = Self::revoke_device_with(store, device_notifier, user_id, token).await?;
        if revoked {
            hub_notifier.notify_hub(user_id, kind, Some(token)).await;
        }
        Ok(revoked)
    }

    pub async fn revoke_device_with<S, N>(
        store: &S,
        notifier: &N,
        user_id: i64,
        token: &str,
    ) -> AppResult<bool>
    where
        S: PairingStore + Sync,
        N: DeviceNotifier + Sync,
    {
        let revoked = store.store_revoke_device_token(user_id, token).await?;
        if revoked {
            notifier.notify_revoked(token).await;
        }
        Ok(revoked)
    }
}
