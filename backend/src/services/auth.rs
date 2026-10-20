#[cfg(not(any(test, coverage)))]
use worker::*;

#[cfg(not(any(test, coverage)))]
use super::store::D1Stores;
use super::store::UserStore;
use crate::models::auth::{GoogleProfile, LoginResponse, TokenInfo, User};
use crate::models::error::{AppError, AppResult};
use crate::utils::{mint_jwt, verify_jwt};

/// Testable Google verifier seam. Production hits Google tokeninfo;
/// tests substitute a fake.
#[allow(async_fn_in_trait)]
pub trait TokenVerifier {
    async fn verify_token(&self, id_token: &str) -> AppResult<GoogleProfile>;
}

/// Production verifier backed by `Env` + Google tokeninfo.
#[cfg(not(any(test, coverage)))]
pub struct EnvVerifier<'a>(pub &'a Env);

#[cfg(not(any(test, coverage)))]
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

        match info.iss.as_deref() {
            Some("accounts.google.com") | Some("https://accounts.google.com") => {}
            _ => {
                return Err(AppError::unauthorized("Invalid Google ID token issuer."));
            }
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
    /// Only used by the sign-in route — no other route accepts Google tokens.
    #[cfg(not(any(test, coverage)))]
    pub async fn verify_google_id_token(env: &Env, id_token: &str) -> AppResult<GoogleProfile> {
        let client_id = crate::utils::env::get_google_client_id(env)?;

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

    /// Verifies the Google token, upserts the user in D1, and mints the
    /// web session JWT the frontend must use from here on.
    #[cfg(not(any(test, coverage)))]
    pub async fn login_with_google(
        env: &Env,
        db: &D1Database,
        id_token: &str,
    ) -> AppResult<LoginResponse> {
        let secret = crate::utils::jwt_secret(env)?;
        let now_secs = Date::now().as_millis() / 1000;
        let verifier = EnvVerifier(env);
        Self::login_with_google_with(&D1Stores(db), &verifier, id_token, &secret, now_secs).await
    }

    pub async fn login_with_google_with<S, V>(
        store: &S,
        verifier: &V,
        id_token: &str,
        jwt_secret: &str,
        now_secs: u64,
    ) -> AppResult<LoginResponse>
    where
        S: UserStore + Sync,
        V: TokenVerifier + Sync,
    {
        let profile = verifier.verify_token(id_token).await?;
        let user = store
            .store_upsert_user(
                &profile.sub,
                &profile.email,
                profile.name.as_deref(),
                profile.picture.as_deref(),
            )
            .await?;
        let token = mint_jwt(&user.google_sub, user.id, jwt_secret, now_secs)
            .map_err(AppError::internal)?;
        Ok(LoginResponse {
            user,
            token,
            expires_at: now_secs
                .saturating_add(crate::common::JWT_EXPIRY_SECS)
                .saturating_mul(1000),
        })
    }

    /// Authenticates every non-sign-in route. Accepts exactly two tokens:
    ///
    /// 1. The server-minted web session JWT (HS256).
    /// 2. A mobile device token (long-lived pairing token).
    ///
    /// Google ID tokens and raw `google_sub` values are rejected here —
    /// a `sub` alone proves nothing, so it must never authenticate.
    #[cfg(not(any(test, coverage)))]
    pub async fn authenticate(token: &str, env: &Env, db: &D1Database) -> AppResult<User> {
        // Fail closed: a missing/short secret is 503, never a silent skip of
        // the JWT path (which would hide misconfiguration as 401s).
        let secret = crate::utils::jwt_secret(env)?;
        let now_secs = Date::now().as_millis() / 1000;
        Self::authenticate_with(&D1Stores(db), token, &secret, now_secs).await
    }

    pub async fn authenticate_with<S>(
        store: &S,
        token: &str,
        jwt_secret: &str,
        now_secs: u64,
    ) -> AppResult<User>
    where
        S: UserStore + Sync,
    {
        // Fail closed when the server secret is missing/short: JWTs cannot be
        // verified, so every request fails with 503 instead of silently
        // degrading to device-credential-only (which masks misconfiguration).
        if jwt_secret.len() < crate::common::MIN_JWT_SECRET_LEN {
            return Err(AppError::service_unavailable(
                "Server auth is not configured.",
            ));
        }
        // 1. Server-minted JWT. The `uid` cross-check binds the token to
        // the exact user row it was issued for.
        if let Ok(claims) = verify_jwt(token, jwt_secret, now_secs) {
            if let Ok(Some(user)) = store.store_get_user_by_sub(&claims.sub).await {
                if user.id == claims.uid {
                    return Ok(user);
                }
            }
            return Err(AppError::unauthorized(
                "Invalid or expired Authorization token.",
            ));
        }

        // 2. Mobile device token.
        if let Ok(Some(user)) = store.store_get_user_by_device_token(token).await {
            return Ok(user);
        }

        Err(AppError::unauthorized(
            "Invalid or expired Authorization token.",
        ))
    }

    /// Fetches a user by their Google subject ID.
    #[cfg(not(any(test, coverage)))]
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
