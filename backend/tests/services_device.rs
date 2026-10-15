use std::collections::HashMap;
use std::sync::Mutex;

use backend::models::auth::User;
use backend::models::device::{DeviceSession, DeviceToken};
use backend::models::error::{AppError, AppResult};
use backend::services::device::DeviceService;
use backend::services::store::{DeviceStore, UserStore};

fn sample_user(id: i64) -> User {
    User {
        id,
        google_sub: format!("sub-{id}"),
        email: format!("u{id}@x.com"),
        name: Some("Test".to_string()),
        picture_url: None,
        created_at: "2026-01-01T00:00:00.000Z".to_string(),
        updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        last_login_at: None,
    }
}

fn sample_session(id: &str, status: &str) -> DeviceSession {
    DeviceSession {
        session_id: id.to_string(),
        short_code: id[..6.min(id.len())].to_lowercase(),
        status: status.to_string(),
        user_id: None,
        device_token: None,
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        updated_at: "u".to_string(),
    }
}

struct FakeStore {
    sessions: Mutex<HashMap<String, DeviceSession>>,
    users: Mutex<HashMap<i64, User>>,
    tokens: Mutex<Vec<DeviceToken>>,
    fail_with: Mutex<Option<String>>,
}

impl FakeStore {
    fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            users: Mutex::new(HashMap::new()),
            tokens: Mutex::new(Vec::new()),
            fail_with: Mutex::new(None),
        }
    }
    fn fail_next(&self, msg: &str) {
        *self.fail_with.lock().unwrap() = Some(msg.to_string());
    }
    fn take_fail(&self) -> Option<AppError> {
        self.fail_with
            .lock()
            .unwrap()
            .take()
            .map(AppError::bad_gateway)
    }
}

impl UserStore for FakeStore {
    async fn store_get_user_by_sub(&self, sub: &str) -> AppResult<Option<User>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.google_sub == sub)
            .cloned())
    }
    async fn store_get_user_by_id(&self, id: i64) -> AppResult<Option<User>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        Ok(self.users.lock().unwrap().get(&id).cloned())
    }
    async fn store_get_user_by_device_token(&self, token: &str) -> AppResult<Option<User>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let _ = token;
        Ok(None)
    }
    async fn store_upsert_user(
        &self,
        sub: &str,
        email: &str,
        name: Option<&str>,
        picture_url: Option<&str>,
    ) -> AppResult<User> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let u = User {
            id: 1,
            google_sub: sub.to_string(),
            email: email.to_string(),
            name: name.map(str::to_string),
            picture_url: picture_url.map(str::to_string),
            created_at: "c".to_string(),
            updated_at: "u".to_string(),
            last_login_at: None,
        };
        self.users.lock().unwrap().insert(1, u.clone());
        Ok(u)
    }
}

impl DeviceStore for FakeStore {
    async fn store_upsert_device_session(
        &self,
        session_id: &str,
        short_code: &str,
    ) -> AppResult<DeviceSession> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let s = DeviceSession {
            session_id: session_id.to_string(),
            short_code: short_code.to_string(),
            status: "pending".to_string(),
            user_id: None,
            device_token: None,
            created_at: "c".to_string(),
            expires_at: "2099-01-01T00:00:00.000Z".to_string(),
            updated_at: "u".to_string(),
        };
        self.sessions.lock().unwrap().insert(session_id.to_string(), s.clone());
        Ok(s)
    }
    async fn store_get_device_session(&self, session_id: &str) -> AppResult<Option<DeviceSession>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        Ok(self.sessions.lock().unwrap().get(session_id).cloned())
    }
    async fn store_find_pending_session(
        &self,
        code_or_session_id: &str,
    ) -> AppResult<Option<DeviceSession>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let lower = code_or_session_id.to_lowercase();
        Ok(self
            .sessions
            .lock()
            .unwrap()
            .values()
            .find(|s| {
                s.status == "pending"
                    && (s.session_id == code_or_session_id
                        || s.short_code.to_lowercase() == lower)
            })
            .cloned())
    }
    async fn store_authorize_device_session(
        &self,
        session_id: &str,
        user_id: i64,
        device_token: &str,
        device_name: &str,
    ) -> AppResult<()> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let mut sessions = self.sessions.lock().unwrap();
        let s = sessions.get_mut(session_id).expect("session must exist");
        s.status = "authorized".to_string();
        s.user_id = Some(user_id);
        s.device_token = Some(device_token.to_string());
        self.tokens.lock().unwrap().push(DeviceToken {
            token: device_token.to_string(),
            user_id,
            device_name: device_name.to_string(),
            created_at: "c".to_string(),
            expires_at: "e".to_string(),
            last_used_at: None,
        });
        Ok(())
    }
    async fn store_get_user_devices(&self, user_id: i64) -> AppResult<Vec<DeviceToken>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        Ok(self
            .tokens
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.user_id == user_id)
            .cloned()
            .collect())
    }
    async fn store_revoke_device_token(&self, user_id: i64, token: &str) -> AppResult<bool> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let mut tokens = self.tokens.lock().unwrap();
        let before = tokens.len();
        tokens.retain(|t| !(t.user_id == user_id && t.token == token));
        Ok(tokens.len() != before)
    }
}

fn valid_session_id() -> String {
    "ab".repeat(64)
}

#[tokio::test]
async fn create_session_ok_uppercases_shortcode() {
    let store = FakeStore::new();
    let sid = valid_session_id();
    let res = DeviceService::create_session_with(&store, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    assert_eq!(res.session_id, sid);
    assert_eq!(res.short_code, sid[..6].to_uppercase());
    assert_eq!(res.qr_svg, "<svg/>");
}

#[tokio::test]
async fn create_session_rejects_invalid() {
    let store = FakeStore::new();
    let err = DeviceService::create_session_with(&store, "bad", |_| Ok(String::new()))
        .await
        .unwrap_err();
    assert_eq!(err.status, 400);
}

#[tokio::test]
async fn create_session_qr_failure_maps_500() {
    let store = FakeStore::new();
    let err = DeviceService::create_session_with(&store, &valid_session_id(), |_| {
        Err("qr boom".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 500);
}

#[tokio::test]
async fn create_session_db_failure_propagates() {
    let store = FakeStore::new();
    store.fail_next("db down");
    let err = DeviceService::create_session_with(&store, &valid_session_id(), |_| {
        Ok("<svg/>".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);
}

#[tokio::test]
async fn authorize_ok_with_defaults_and_custom_name() {
    let store = FakeStore::new();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();

    let res = DeviceService::authorize_device_with(&store, 42, &sid, None, || {
        Ok("tok123".to_string())
    })
    .await
    .unwrap();
    assert_eq!(res.status, "authorized");
    assert_eq!(res.device_name, "Mobile Device");

    // Second session with custom name and short-code lookup.
    let sid2 = "cd".repeat(64);
    let created = DeviceService::create_session_with(&store, &sid2, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    let res2 = DeviceService::authorize_device_with(
        &store,
        42,
        &created.short_code,
        Some("  Pixel  "),
        || Ok("tok2".to_string()),
    )
    .await
    .unwrap();
    assert_eq!(res2.device_name, "Pixel");
}

#[tokio::test]
async fn authorize_rejects_empty_and_missing() {
    let store = FakeStore::new();
    let err = DeviceService::authorize_device_with(&store, 1, "   ", None, || {
        Ok("t".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 400);

    let err = DeviceService::authorize_device_with(&store, 1, "nonexistent", None, || {
        Ok("t".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 404);
}

#[tokio::test]
async fn authorize_token_gen_failure_maps_500() {
    let store = FakeStore::new();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    let err = DeviceService::authorize_device_with(&store, 1, &sid, None, || {
        Err("rng".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 500);
}

#[tokio::test]
async fn check_session_pending_authorized_expired() {
    let store = FakeStore::new();
    // Missing -> 400, unknown -> 404.
    assert_eq!(
        DeviceService::check_session_with(&store, "  ")
            .await
            .unwrap_err()
            .status,
        400
    );
    assert_eq!(
        DeviceService::check_session_with(&store, "nope")
            .await
            .unwrap_err()
            .status,
        404
    );

    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    let pending = DeviceService::check_session_with(&store, &sid).await.unwrap();
    assert_eq!(pending.status, "pending");
    assert!(pending.token.is_none());

    // Authorize then poll returns token + user.
    store.users.lock().unwrap().insert(9, sample_user(9));
    // Manually mark authorized with user 9.
    {
        let mut sessions = store.sessions.lock().unwrap();
        let s = sessions.get_mut(&sid).unwrap();
        s.status = "authorized".to_string();
        s.user_id = Some(9);
        s.device_token = Some("tok-9".to_string());
    }
    let got = DeviceService::check_session_with(&store, &sid).await.unwrap();
    assert_eq!(got.status, "authorized");
    assert_eq!(got.token.as_deref(), Some("tok-9"));
    assert_eq!(got.user.map(|u| u.id), Some(9));

    // Expired passthrough.
    let mut exp = sample_session(&"ef".repeat(64), "expired");
    exp.session_id = "expsess".to_string();
    store
        .sessions
        .lock()
        .unwrap()
        .insert("expsess".to_string(), exp);
    let got = DeviceService::check_session_with(&store, "expsess")
        .await
        .unwrap();
    assert_eq!(got.status, "expired");
}

#[tokio::test]
async fn get_and_revoke_devices() {
    let store = FakeStore::new();
    assert!(DeviceService::get_user_devices_with(&store, 1)
        .await
        .unwrap()
        .is_empty());
    store.tokens.lock().unwrap().push(DeviceToken {
        token: "t1".to_string(),
        user_id: 1,
        device_name: "Mobile Device".to_string(),
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        last_used_at: None,
    });
    assert_eq!(
        DeviceService::get_user_devices_with(&store, 1)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(DeviceService::revoke_device_with(&store, 1, "t1")
        .await
        .unwrap());
    assert!(!DeviceService::revoke_device_with(&store, 1, "t1")
        .await
        .unwrap());

    store.fail_next("db");
    assert_eq!(
        DeviceService::get_user_devices_with(&store, 1)
            .await
            .unwrap_err()
            .status,
        502
    );
}
