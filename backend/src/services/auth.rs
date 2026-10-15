use worker::*;

use super::store::{D1Stores, UserStore};
use crate::models::auth::{GoogleProfile, TokenInfo, User};
use crate::models::error::{AppError, AppResult};

/// Testable Google verifier seam. Production hits Google tokeninfo;
/// tests substitute a fake.
#[allow(async_fn_in_trait)]
pub trait TokenVerifier {
    async fn verify_token(&self, id_token: &str) -> AppResult<GoogleProfile>;
}

/// Production verifier backed by `Env` + Google tokeninfo.
pub struct EnvVerifier<'a>(pub &'a Env);

impl TokenVerifier for EnvVerifier<'_> {
    async fn verify_token(&self, id_token: &str) -> AppResult<GoogleProfile> {
        AuthService::verify_google_id_token(self.0, id_token).await
    }
}

pub struct AuthService;

impl AuthService {
    /// Pure TokenInfo validation shared by production and tests.
    pub fn validate_token_info(
        info: TokenInfo,
        client_id: &str,
        now_millis: u64,
    ) -> AppResult<GoogleProfile> {
        let sub = info.sub.filter(|s| !s.is_empty());
        let email = info.email.filter(|s| !s.is_empty());
        let (sub, email) = match (sub, email) {
            (Some(sub), Some(email)) => (sub, email),
            _ => return Err(AppError::unauthorized("Invalid Google ID token.")),
        };

        if info.aud.as_deref() != Some(client_id) {
            return Err(AppError::unauthorized(
                "Google ID token was not issued for this app.",
            ));
        }

        let exp = info
            .exp
            .as_deref()
            .ok_or_else(|| AppError::unauthorized("Invalid Google ID token."))?;
        let exp_seconds = exp
            .parse::<u64>()
            .map_err(|_| AppError::unauthorized("Invalid Google ID token."))?;

        if exp_seconds.saturating_mul(1000) < now_millis {
            return Err(AppError::unauthorized("Google ID token has expired."));
        }

        Ok(GoogleProfile {
            sub,
            email,
            name: info.name,
            picture: info.picture,
        })
    }

    /// Verifies a Google ID token with Google's tokeninfo endpoint.
    pub async fn verify_google_id_token(env: &Env, id_token: &str) -> AppResult<GoogleProfile> {
        let client_id = env
            .secret("GOOGLE_CLIENT_ID")
            .map(|secret| secret.to_string())
            .map_err(|_| {
                AppError::service_unavailable("Google login is not configured on the server.")
            })?;

        if id_token.is_empty() {
            return Err(AppError::bad_request("Missing Google ID token."));
        }

        let encoded: String = url::form_urlencoded::byte_serialize(id_token.as_bytes()).collect();
        let url: url::Url = format!("https://oauth2.googleapis.com/tokeninfo?id_token={encoded}")
            .parse()
            .map_err(|_| AppError::bad_gateway("Failed to verify Google ID token."))?;

        let mut res = Fetch::Url(url)
            .send()
            .await
            .map_err(|_| AppError::bad_gateway("Failed to reach Google token verification."))?;

        if res.status_code() != 200 {
            return Err(AppError::unauthorized("Invalid Google ID token."));
        }

        let info: TokenInfo = res
            .json()
            .await
            .map_err(|_| AppError::unauthorized("Invalid Google ID token."))?;

        Self::validate_token_info(info, &client_id, Date::now().as_millis())
    }

    /// Verifies the Google token and creates/updates the user profile in D1.
    pub async fn login_with_google(env: &Env, db: &D1Database, id_token: &str) -> AppResult<User> {
        let verifier = EnvVerifier(env);
        Self::login_with_google_with(&D1Stores(db), &verifier, id_token).await
    }

    pub async fn login_with_google_with<S, V>(
        store: &S,
        verifier: &V,
        id_token: &str,
    ) -> AppResult<User>
    where
        S: UserStore + Sync,
        V: TokenVerifier + Sync,
    {
        let profile = verifier.verify_token(id_token).await?;
        store
            .store_upsert_user(
                &profile.sub,
                &profile.email,
                profile.name.as_deref(),
                profile.picture.as_deref(),
            )
            .await
    }

    /// Unified authentication mechanism:
    /// 1. Device token (mobile long-lived session)
    /// 2. Google ID token (web session)
    /// 3. Direct sub lookup (dev/testing)
    pub async fn authenticate(token: &str, env: &Env, db: &D1Database) -> AppResult<User> {
        let verifier = EnvVerifier(env);
        Self::authenticate_with(&D1Stores(db), &verifier, token).await
    }

    pub async fn authenticate_with<S, V>(store: &S, verifier: &V, token: &str) -> AppResult<User>
    where
        S: UserStore + Sync,
        V: TokenVerifier + Sync,
    {
        // 1. Try active device token
        if let Ok(Some(user)) = store.store_get_user_by_device_token(token).await {
            return Ok(user);
        }

        // 2. Try Google ID token
        if let Ok(profile) = verifier.verify_token(token).await {
            if let Ok(Some(user)) = store.store_get_user_by_sub(&profile.sub).await {
                return Ok(user);
            }
        }

        // 3. Fallback direct sub lookup (useful for local development/testing)
        if let Ok(Some(user)) = store.store_get_user_by_sub(token).await {
            return Ok(user);
        }

        Err(AppError::unauthorized(
            "Invalid or expired Authorization token.",
        ))
    }

    /// Fetches a user by their Google subject ID.
    pub async fn get_user_by_sub(db: &D1Database, sub: &str) -> AppResult<User> {
        Self::get_user_by_sub_with(&D1Stores(db), sub).await
    }

    pub async fn get_user_by_sub_with<S>(store: &S, sub: &str) -> AppResult<User>
    where
        S: UserStore + Sync,
    {
        let user = store.store_get_user_by_sub(sub).await.map_err(|_| {
            AppError::bad_gateway("Failed to load user.")
        })?;

        user.ok_or_else(|| AppError::not_found("User not found."))
    }
}
