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
    TokenInfo { aud: aud.map(str::to_string), sub: sub.map(str::to_string), email: email.map(str::to_string), name: None, picture: None, exp: exp.map(str::to_string) }
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

#[tokio::test]
async fn authenticate_device_token_wins() {
    let store = FakeUsers::new();
    let verifier = FakeVerifier { profile: None };
    let u = AuthService::authenticate_with(&store, &verifier, "device-good")
        .await
        .unwrap();
    assert_eq!(u.id, 11);
}

#[tokio::test]
async fn authenticate_google_then_sub_fallback() {
    let store = FakeUsers::with_user(user(2, "sub-g"));
    let verifier = FakeVerifier { profile: Some(profile("sub-g")) };
    let u = AuthService::authenticate_with(&store, &verifier, "google-token")
        .await
        .unwrap();
    assert_eq!(u.id, 2);

    // Direct sub fallback when verifier fails.
    let store2 = FakeUsers::with_user(user(3, "direct-sub"));
    let bad = FakeVerifier { profile: None };
    let u = AuthService::authenticate_with(&store2, &bad, "direct-sub")
        .await
        .unwrap();
    assert_eq!(u.id, 3);
}

#[tokio::test]
async fn authenticate_all_miss_401() {
    let store = FakeUsers::new();
    let verifier = FakeVerifier { profile: None };
    let err = AuthService::authenticate_with(&store, &verifier, "nope")
        .await
        .unwrap_err();
    assert_eq!(err.status, 401);
}

#[tokio::test]
async fn login_with_google_upserts() {
    let store = FakeUsers::new();
    let verifier = FakeVerifier { profile: Some(profile("new-sub")) };
    let u = AuthService::login_with_google_with(&store, &verifier, "tok")
        .await
        .unwrap();
    assert_eq!(u.google_sub, "new-sub");

    let bad = FakeVerifier { profile: None };
    assert_eq!(
        AuthService::login_with_google_with(&store, &bad, "tok")
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
