use std::collections::HashMap;
use std::sync::Mutex;

use backend::models::auth::{GoogleProfile, TokenInfo, User};
use backend::models::error::{AppError, AppResult};
use backend::services::auth::{AuthService, TokenVerifier};
use backend::services::store::UserStore;

fn user(id: i64, sub: &str) -> User {
    User {
        id,
        google_sub: sub.to_string(),
        email: format!("{sub}@x.com"),
        name: None,
        picture_url: None,
        created_at: "c".to_string(),
        updated_at: "u".to_string(),
        last_login_at: None,
    }
}

struct FakeUsers {
    by_sub: Mutex<HashMap<String, User>>,
    fail: Mutex<bool>,
}

impl FakeUsers {
    fn new() -> Self {
        Self { by_sub: Mutex::new(HashMap::new()), fail: Mutex::new(false) }
    }
    fn with_user(u: User) -> Self {
        let s = Self::new();
        s.by_sub.lock().unwrap().insert(u.google_sub.clone(), u);
        s
    }
}

impl UserStore for FakeUsers {
    async fn store_get_user_by_sub(&self, sub: &str) -> AppResult<Option<User>> {
        if *self.fail.lock().unwrap() {
            return Err(AppError::bad_gateway("db"));
        }
        Ok(self.by_sub.lock().unwrap().get(sub).cloned())
    }
    async fn store_get_user_by_id(&self, _id: i64) -> AppResult<Option<User>> {
        Ok(None)
    }
    async fn store_get_user_by_device_token(&self, token: &str) -> AppResult<Option<User>> {
        if token == "device-good" {
            return Ok(Some(user(11, "sub-device")));
        }
        if token == "device-err" {
            return Err(AppError::bad_gateway("db"));
        }
        Ok(None)
    }
    async fn store_upsert_user(
        &self,
        sub: &str,
        email: &str,
        name: Option<&str>,
        picture_url: Option<&str>,
    ) -> AppResult<User> {
        if *self.fail.lock().unwrap() {
            return Err(AppError::bad_gateway("save"));
        }
        let u = User {
            id: 99,
            google_sub: sub.to_string(),
            email: email.to_string(),
            name: name.map(str::to_string),
            picture_url: picture_url.map(str::to_string),
            created_at: "c".to_string(),
            updated_at: "u".to_string(),
            last_login_at: None,
        };
        self.by_sub.lock().unwrap().insert(sub.to_string(), u.clone());
        Ok(u)
    }
}

struct FakeVerifier {
    profile: Option<GoogleProfile>,
}

impl TokenVerifier for FakeVerifier {
    async fn verify_token(&self, _token: &str) -> AppResult<GoogleProfile> {
        self.profile.clone().ok_or_else(|| AppError::unauthorized("bad"))
    }
}

fn profile(sub: &str) -> GoogleProfile {
    GoogleProfile { sub: sub.to_string(), email: format!("{sub}@x.com"), name: None, picture: None }
}

fn token_info(aud: Option<&str>, sub: Option<&str>, email: Option<&str>, exp: Option<&str>) -> TokenInfo {
    TokenInfo { aud: aud.map(str::to_string), iss: Some("accounts.google.com".to_string()), sub: sub.map(str::to_string), email: email.map(str::to_string), name: None, picture: None, exp: exp.map(str::to_string) }
}

#[test]
fn validate_token_info_ok() {
    let p = AuthService::validate_token_info(
        token_info(Some("cid"), Some("s"), Some("e@x.com"), Some("9999999999")),
        "cid",
        1_000,
    )
    .unwrap();
    assert_eq!(p.sub, "s");
}

#[test]
fn validate_token_info_rejects_bad_issuer() {
    let mut bad = token_info(Some("cid"), Some("s"), Some("e@x.com"), Some("9999999999"));
    bad.iss = Some("evil.example.com".to_string());
    assert_eq!(
        AuthService::validate_token_info(bad, "cid", 1_000).unwrap_err().status,
        401
    );
    let mut missing = token_info(Some("cid"), Some("s"), Some("e@x.com"), Some("9999999999"));
    missing.iss = None;
    assert_eq!(
        AuthService::validate_token_info(missing, "cid", 1_000).unwrap_err().status,
        401
    );
    // https variant is accepted.
    let mut https = token_info(Some("cid"), Some("s"), Some("e@x.com"), Some("9999999999"));
    https.iss = Some("https://accounts.google.com".to_string());
    assert!(AuthService::validate_token_info(https, "cid", 1_000).is_ok());
}

#[test]
fn validate_token_info_rejects() {
    // Missing sub/email.
    assert_eq!(
        AuthService::validate_token_info(
            token_info(Some("cid"), None, Some("e"), Some("9999999999")),
            "cid",
            0
        )
        .unwrap_err()
        .status,
        401
    );
    assert_eq!(
        AuthService::validate_token_info(
            token_info(Some("cid"), Some(""), Some("e"), Some("9999999999")),
            "cid",
            0
        )
        .unwrap_err()
        .status,
        401
    );
    // Wrong audience.
    assert!(AuthService::validate_token_info(
        token_info(Some("other"), Some("s"), Some("e"), Some("9999999999")),
        "cid",
        0
    )
    .unwrap_err()
    .message
    .contains("not issued"));
    // Missing/bad exp.
    assert_eq!(
        AuthService::validate_token_info(token_info(Some("cid"), Some("s"), Some("e"), None), "cid", 0)
            .unwrap_err()
            .status,
        401
    );
    assert_eq!(
        AuthService::validate_token_info(
            token_info(Some("cid"), Some("s"), Some("e"), Some("nan")),
            "cid",
            0
        )
        .unwrap_err()
        .status,
        401
    );
    // Expired.
    assert_eq!(
        AuthService::validate_token_info(
            token_info(Some("cid"), Some("s"), Some("e"), Some("1")),
            "cid",
            5_000
        )
        .unwrap_err()
        .status,
        401
    );
}

const JWT_SECRET: &str = "test-secret-for-auth-suite-32chars!!";
const NOW_SECS: u64 = 1_700_000_000;

#[tokio::test]
async fn authenticate_jwt_wins() {
    let store = FakeUsers::with_user(user(2, "sub-jwt"));
    let token = backend::utils::mint_jwt("sub-jwt", 2, JWT_SECRET, NOW_SECS).unwrap();
    let u = AuthService::authenticate_with(&store, &token, JWT_SECRET, NOW_SECS)
        .await
        .unwrap();
    assert_eq!(u.id, 2);
}

#[tokio::test]
async fn authenticate_device_token_still_works() {
    let store = FakeUsers::new();
    let u = AuthService::authenticate_with(&store, "device-good", JWT_SECRET, NOW_SECS)
        .await
        .unwrap();
    assert_eq!(u.id, 11);
}

#[tokio::test]
async fn authenticate_raw_sub_never_authenticates() {
    // Regression: a bare google_sub proves nothing and must not log in,
    // even when it names a real user.
    let store = FakeUsers::with_user(user(3, "direct-sub"));
    let err = AuthService::authenticate_with(&store, "direct-sub", JWT_SECRET, NOW_SECS)
        .await
        .unwrap_err();
    assert_eq!(err.status, 401);
}

#[tokio::test]
async fn authenticate_rejects_forged_and_stale_tokens() {
    // JWT for an unknown sub: no auto-provisioning outside sign-in.
    let store = FakeUsers::new();
    let ghost = backend::utils::mint_jwt("ghost", 99, JWT_SECRET, NOW_SECS).unwrap();
    assert_eq!(
        AuthService::authenticate_with(&store, &ghost, JWT_SECRET, NOW_SECS)
            .await
            .unwrap_err()
            .status,
        401
    );

    // uid mismatch: token bound to a different row.
    let store = FakeUsers::with_user(user(4, "sub-4"));
    let swapped = backend::utils::mint_jwt("sub-4", 999, JWT_SECRET, NOW_SECS).unwrap();
    assert_eq!(
        AuthService::authenticate_with(&store, &swapped, JWT_SECRET, NOW_SECS)
            .await
            .unwrap_err()
            .status,
        401
    );

    // Expired JWT.
    let store = FakeUsers::with_user(user(5, "sub-5"));
    let stale = backend::utils::mint_jwt("sub-5", 5, JWT_SECRET, NOW_SECS).unwrap();
    assert_eq!(
        AuthService::authenticate_with(&store, &stale, JWT_SECRET, NOW_SECS + 7200)
            .await
            .unwrap_err()
            .status,
        401
    );

    // Wrong secret and garbage both fall through to a 401.
    assert_eq!(
        AuthService::authenticate_with(&store, &stale, "other-secret-32chars-long-wrong!!", NOW_SECS)
            .await
            .unwrap_err()
            .status,
        401
    );
    assert_eq!(
        AuthService::authenticate_with(&store, "nope", JWT_SECRET, NOW_SECS)
            .await
            .unwrap_err()
            .status,
        401
    );
}

#[tokio::test]
async fn authenticate_fails_closed_without_secret() {
    let store = FakeUsers::with_user(user(6, "sub-6"));
    for bad in ["", "short"] {
        assert_eq!(
            AuthService::authenticate_with(&store, "device-good", bad, NOW_SECS)
                .await
                .unwrap_err()
                .status,
            503
        );
    }
}

#[tokio::test]
async fn login_with_google_upserts_and_mints_verifiable_jwt() {
    let store = FakeUsers::new();
    let verifier = FakeVerifier { profile: Some(profile("new-sub")) };
    let login =
        AuthService::login_with_google_with(&store, &verifier, "tok", JWT_SECRET, NOW_SECS)
            .await
            .unwrap();
    assert_eq!(login.user.google_sub, "new-sub");
    assert_eq!(
        login.expires_at,
        (NOW_SECS + backend::common::JWT_EXPIRY_SECS) * 1000
    );

    let claims =
        backend::utils::verify_jwt(&login.token, JWT_SECRET, NOW_SECS).unwrap();
    assert_eq!(claims.sub, "new-sub");
    assert_eq!(claims.uid, login.user.id);

    // The minted token immediately authenticates.
    let u = AuthService::authenticate_with(&store, &login.token, JWT_SECRET, NOW_SECS)
        .await
        .unwrap();
    assert_eq!(u.google_sub, "new-sub");

    let bad = FakeVerifier { profile: None };
    assert_eq!(
        AuthService::login_with_google_with(&store, &bad, "tok", JWT_SECRET, NOW_SECS)
            .await
            .unwrap_err()
            .status,
        401
    );
}

#[tokio::test]
async fn get_user_by_sub_found_missing_and_db_error() {
    let store = FakeUsers::with_user(user(5, "s5"));
    let u = AuthService::get_user_by_sub_with(&store, "s5").await.unwrap();
    assert_eq!(u.id, 5);
    assert_eq!(
        AuthService::get_user_by_sub_with(&store, "missing")
            .await
            .unwrap_err()
            .status,
        404
    );
    *store.fail.lock().unwrap() = true;
    assert_eq!(
        AuthService::get_user_by_sub_with(&store, "s5")
            .await
            .unwrap_err()
            .status,
        502
    );
}
