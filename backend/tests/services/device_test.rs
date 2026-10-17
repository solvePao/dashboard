use std::collections::HashMap;
use std::sync::Mutex;

use backend::common::DeviceLifecycleKind;
use backend::models::auth::User;
use backend::models::device::{CheckSessionResponse, DeviceToken, PairingCode, PairingState};
use backend::models::error::{AppError, AppResult};
use backend::services::device::DeviceService;
use backend::services::store::{DeviceNotifier, HubNotifier, NoNotify, PairingClient, PairingStore};

fn sample_user(id: i64) -> User {
    User {
        id,
        google_sub: format!("sub-{id}"),
        email: format!("u{id}@x.com"),
        name: Some("Test".to_string()),
        picture_url: None,
        created_at: "c".to_string(),
        updated_at: "u".to_string(),
        last_login_at: None,
    }
}

fn pending_state(expires: &str) -> PairingState {
    PairingState {
        status: "pending".to_string(),
        expires_at: expires.to_string(),
        token: None,
        user: None,
    }
}

fn as_check(state: &PairingState) -> CheckSessionResponse {
    let authorized = state.status == "authorized";
    CheckSessionResponse {
        status: state.status.clone(),
        token: if authorized { state.token.clone() } else { None },
        user: if authorized { state.user.clone() } else { None },
    }
}

struct FakePairingStore {
    codes: Mutex<HashMap<String, PairingCode>>,
    tokens: Mutex<Vec<DeviceToken>>,
    fail_with: Mutex<Option<String>>,
    fail_insert: Mutex<bool>,
}

impl FakePairingStore {
    fn new() -> Self {
        Self {
            codes: Mutex::new(HashMap::new()),
            tokens: Mutex::new(Vec::new()),
            fail_with: Mutex::new(None),
            fail_insert: Mutex::new(false),
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

impl PairingStore for FakePairingStore {
    async fn store_upsert_pairing_code(
        &self,
        session_id: &str,
        short_code: &str,
    ) -> AppResult<PairingCode> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let row = PairingCode {
            session_id: session_id.to_string(),
            short_code: short_code.to_string(),
            expires_at: "2099-01-01T00:00:00.000Z".to_string(),
        };
        self.codes.lock().unwrap().insert(session_id.to_string(), row.clone());
        Ok(row)
    }
    async fn store_find_session_by_code(&self, code: &str) -> AppResult<Option<PairingCode>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let lower = code.to_lowercase();
        Ok(self
            .codes
            .lock()
            .unwrap()
            .values()
            .find(|c| c.short_code.to_lowercase() == lower)
            .cloned())
    }
    async fn store_get_pairing_code(&self, session_id: &str) -> AppResult<Option<PairingCode>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        Ok(self.codes.lock().unwrap().get(session_id).cloned())
    }
    async fn store_insert_device_token(
        &self,
        user_id: i64,
        device_token: &str,
        device_name: &str,
    ) -> AppResult<()> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        if *self.fail_insert.lock().unwrap() {
            return Err(AppError::bad_gateway("insert down"));
        }
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
    async fn store_delete_device_token(&self, device_token: &str) -> AppResult<()> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        self.tokens.lock().unwrap().retain(|t| t.token != device_token);
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

struct FakePairingClient {
    states: Mutex<HashMap<String, PairingState>>,
    fail_with: Mutex<Option<String>>,
    refuse_commit: Mutex<bool>,
    fail_commit: Mutex<bool>,
}

impl FakePairingClient {
    fn new() -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
            fail_with: Mutex::new(None),
            refuse_commit: Mutex::new(false),
            fail_commit: Mutex::new(false),
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
    fn seed(&self, session_id: &str, state: PairingState) {
        self.states.lock().unwrap().insert(session_id.to_string(), state);
    }
}

impl PairingClient for FakePairingClient {
    async fn client_init(&self, session_id: &str, expires_at: &str) -> AppResult<PairingState> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        let mut states = self.states.lock().unwrap();
        if let Some(existing) = states.get(session_id) {
            if existing.status == "authorized" {
                return Ok(existing.clone());
            }
        }
        let state = pending_state(expires_at);
        states.insert(session_id.to_string(), state.clone());
        Ok(state)
    }
    async fn client_status(&self, session_id: &str) -> AppResult<Option<CheckSessionResponse>> {
        if let Some(e) = self.take_fail() {
            return Err(e);
        }
        Ok(self.states.lock().unwrap().get(session_id).map(as_check))
    }
    async fn client_commit(
        &self,
        session_id: &str,
        device_token: &str,
        user: &User,
    ) -> AppResult<bool> {
        if *self.fail_commit.lock().unwrap() {
            return Err(AppError::bad_gateway("commit down"));
        }
        if *self.refuse_commit.lock().unwrap() {
            return Ok(false);
        }
        let current = self.states.lock().unwrap().get(session_id).cloned();
        match current {
            None => Err(AppError::not_found("Pending session not found or expired.")),
            Some(s) if s.status == "authorized" => Ok(false),
            Some(s) if s.status != "pending" => {
                Err(AppError::not_found("Pending session not found or expired."))
            }
            Some(s) => {
                self.states.lock().unwrap().insert(
                    session_id.to_string(),
                    PairingState {
                        status: "authorized".to_string(),
                        expires_at: s.expires_at,
                        token: Some(device_token.to_string()),
                        user: Some(user.clone()),
                    },
                );
                Ok(true)
            }
        }
    }
}

struct FakeNotifier {
    calls: Mutex<Vec<String>>,
}

impl FakeNotifier {
    fn new() -> Self {
        Self { calls: Mutex::new(Vec::new()) }
    }
}

impl DeviceNotifier for FakeNotifier {
    async fn notify_revoked(&self, device_token: &str) {
        self.calls.lock().unwrap().push(device_token.to_string());
    }
}

struct FakeHubNotifier {
    calls: Mutex<Vec<(i64, DeviceLifecycleKind, Option<String>)>>,
}

impl FakeHubNotifier {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl HubNotifier for FakeHubNotifier {
    async fn notify_hub(&self, user_id: i64, kind: DeviceLifecycleKind, device_token: Option<&str>) {
        self.calls.lock().unwrap().push((user_id, kind, device_token.map(str::to_string)));
    }
}

fn valid_session_id() -> String {
    "ab".repeat(16)
}

fn harness() -> (FakePairingStore, FakePairingClient) {
    (FakePairingStore::new(), FakePairingClient::new())
}

#[tokio::test]
async fn create_session_ok_uppercases_shortcode() {
    let (store, client) = harness();
    let sid = valid_session_id();
    let res = DeviceService::create_session_with(&store, &client, &sid, |_| {
        Ok("<svg/>".to_string())
    })
    .await
    .unwrap();
    assert_eq!(res.session_id, sid);
    assert_eq!(res.short_code, sid[..6].to_uppercase());
    assert_eq!(res.qr_svg, "<svg/>");
    assert!(!res.expires_at.is_empty());
}

#[tokio::test]
async fn create_session_rejects_invalid() {
    let (store, client) = harness();
    let err = DeviceService::create_session_with(&store, &client, "bad", |_| Ok(String::new()))
        .await
        .unwrap_err();
    assert_eq!(err.status, 400);
}

#[tokio::test]
async fn create_session_qr_failure_maps_500() {
    let (store, client) = harness();
    let err = DeviceService::create_session_with(&store, &client, &valid_session_id(), |_| {
        Err("qr boom".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 500);
}

#[tokio::test]
async fn create_session_db_failure_propagates() {
    let (store, client) = harness();
    store.fail_next("db down");
    let err = DeviceService::create_session_with(&store, &client, &valid_session_id(), |_| {
        Ok("<svg/>".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);
}

#[tokio::test]
async fn create_session_client_failure_propagates() {
    let (store, client) = harness();
    client.fail_next("do down");
    let err = DeviceService::create_session_with(&store, &client, &valid_session_id(), |_| {
        Ok("<svg/>".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);
}

#[tokio::test]
async fn create_session_never_reopens_authorized() {
    let (store, client) = harness();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    let user = sample_user(42);
    DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Ok("tok-first".to_string())
    })
    .await
    .unwrap();
    // Re-creating a finished pairing returns it instead of resetting it.
    let again = DeviceService::create_session_with(&store, &client, &sid, |_| {
        Ok("<svg/>".to_string())
    })
    .await
    .unwrap();
    assert_eq!(again.session_id, sid);
    let status = DeviceService::check_session_with(&client, &sid).await.unwrap();
    assert_eq!(status.status, "authorized");
    assert_eq!(status.token.as_deref(), Some("tok-first"));
}

#[tokio::test]
async fn authorize_ok_with_defaults_and_custom_name() {
    let (store, client) = harness();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    let user = sample_user(42);

    let res = DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Ok("tok123".to_string())
    })
    .await
    .unwrap();
    assert_eq!(res.status, "authorized");
    assert_eq!(res.device_name, "Mobile Device");

    // Second session with custom name and short-code lookup.
    let sid2 = "cd".repeat(16);
    let created =
        DeviceService::create_session_with(&store, &client, &sid2, |_| Ok("<svg/>".to_string()))
            .await
            .unwrap();
    let res2 = DeviceService::authorize_device_with(
        &store,
        &client,
        &user,
        &created.short_code,
        Some("  Pixel  "),
        || Ok("tok2".to_string()),
    )
    .await
    .unwrap();
    assert_eq!(res2.device_name, "Pixel");
}

#[tokio::test]
async fn authorize_second_attempt_conflicts_without_overwrite() {
    let (store, client) = harness();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    let first = sample_user(42);
    DeviceService::authorize_device_with(&store, &client, &first, &sid, None, || {
        Ok("tok-first".to_string())
    })
    .await
    .unwrap();
    // Same session, different user → loser gets 409 and the winner's binding is preserved.
    let second = sample_user(43);
    let err = DeviceService::authorize_device_with(&store, &client, &second, &sid, None, || {
        Ok("tok-second".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 409);
    let status = DeviceService::check_session_with(&client, &sid).await.unwrap();
    assert_eq!(status.user.map(|u| u.id), Some(42));
    assert_eq!(status.token.as_deref(), Some("tok-first"));
    // Loser's token was never persisted.
    assert!(
        store.tokens.lock().unwrap().iter().all(|t| t.token != "tok-second"),
        "loser token must be compensated"
    );
}

#[tokio::test]
async fn authorize_refused_commit_compensates_token() {
    let (store, client) = harness();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    // Status still reads pending, but the commit loses the race.
    *client.refuse_commit.lock().unwrap() = true;
    let user = sample_user(7);
    let err = DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Ok("tok-race".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 409);
    assert!(store.tokens.lock().unwrap().is_empty());
}

#[tokio::test]
async fn authorize_commit_error_compensates_token() {
    let (store, client) = harness();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    *client.fail_commit.lock().unwrap() = true;
    let user = sample_user(7);
    let err = DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Ok("tok-err".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);
    assert!(store.tokens.lock().unwrap().is_empty());
}

#[tokio::test]
async fn authorize_rejects_empty_missing_and_expired() {
    let (store, client) = harness();
    let user = sample_user(1);
    let err = DeviceService::authorize_device_with(&store, &client, &user, "   ", None, || {
        Ok("t".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 400);

    let err = DeviceService::authorize_device_with(&store, &client, &user, "nonexistent", None, || {
        Ok("t".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 404);

    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    client.seed(
        &sid,
        PairingState {
            status: "expired".to_string(),
            expires_at: "e".to_string(),
            token: None,
            user: None,
        },
    );
    let err = DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Ok("t".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 404);
    assert!(store.tokens.lock().unwrap().is_empty());
}

#[tokio::test]
async fn create_session_reuse_propagates_store_failures() {
    let (store, client) = harness();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    let user = sample_user(42);
    DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Ok("tok-first".to_string())
    })
    .await
    .unwrap();
    // Re-create hits the authorized-reuse path: missing index row is 404.
    store.codes.lock().unwrap().remove(&sid);
    let err = DeviceService::create_session_with(&store, &client, &sid, |_| {
        Ok("<svg/>".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 404);

    // A failing index read on the reuse path is 502.
    store.codes.lock().unwrap().insert(
        sid.clone(),
        backend::models::device::PairingCode {
            session_id: sid.clone(),
            short_code: sid[..6].to_string(),
            expires_at: "e".to_string(),
        },
    );
    store.fail_next("index down");
    let err = DeviceService::create_session_with(&store, &client, &sid, |_| {
        Ok("<svg/>".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);
}

#[tokio::test]
async fn authorize_propagates_store_and_client_failures() {
    let user = sample_user(1);
    // Code lookup failure.
    let (store, client) = harness();
    store.fail_next("index down");
    let err = DeviceService::authorize_device_with(&store, &client, &user, "abcdef", None, || {
        Ok("t".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);

    // Status read failure (full session id skips the index).
    let (store, client) = harness();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    client.fail_next("do down");
    let err = DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Ok("t".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);

    // Token insert failure persists nothing.
    let (store, client) = harness();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    *store.fail_insert.lock().unwrap() = true;
    let err = DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Ok("tok-x".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 502);
    assert!(store.tokens.lock().unwrap().is_empty());
}

#[tokio::test]
async fn authorize_token_gen_failure_maps_500() {
    let (store, client) = harness();
    let sid = valid_session_id();
    DeviceService::create_session_with(&store, &client, &sid, |_| Ok("<svg/>".to_string()))
        .await
        .unwrap();
    let user = sample_user(1);
    let err = DeviceService::authorize_device_with(&store, &client, &user, &sid, None, || {
        Err("rng".to_string())
    })
    .await
    .unwrap_err();
    assert_eq!(err.status, 500);
}

#[tokio::test]
async fn check_session_pending_authorized_expired() {
    let (_store, client) = harness();
    // Missing -> 400, unknown -> 404.
    assert_eq!(
        DeviceService::check_session_with(&client, "  ").await.unwrap_err().status,
        400
    );
    assert_eq!(
        DeviceService::check_session_with(&client, "nope").await.unwrap_err().status,
        404
    );

    let sid = valid_session_id();
    client.seed(&sid, pending_state("2099-01-01T00:00:00.000Z"));
    let pending = DeviceService::check_session_with(&client, &sid).await.unwrap();
    assert_eq!(pending.status, "pending");
    assert!(pending.token.is_none());

    // Authorized returns token + user; pending views never leak them.
    client.seed(
        &sid,
        PairingState {
            status: "authorized".to_string(),
            expires_at: "e".to_string(),
            token: Some("tok-9".to_string()),
            user: Some(sample_user(9)),
        },
    );
    let got = DeviceService::check_session_with(&client, &sid).await.unwrap();
    assert_eq!(got.status, "authorized");
    assert_eq!(got.token.as_deref(), Some("tok-9"));
    assert_eq!(got.user.map(|u| u.id), Some(9));

    client.seed(
        "expsess",
        PairingState {
            status: "expired".to_string(),
            expires_at: "e".to_string(),
            token: None,
            user: None,
        },
    );
    let got = DeviceService::check_session_with(&client, "expsess").await.unwrap();
    assert_eq!(got.status, "expired");
    assert!(got.token.is_none());
}

#[tokio::test]
async fn check_session_client_failure_propagates() {
    let (_store, client) = harness();
    client.fail_next("do down");
    let err = DeviceService::check_session_with(&client, &valid_session_id())
        .await
        .unwrap_err();
    assert_eq!(err.status, 502);
}

#[tokio::test]
async fn get_and_revoke_devices() {
    let (store, _client) = harness();
    let notifier = FakeNotifier::new();
    assert!(DeviceService::get_user_devices_with(&store, 1).await.unwrap().is_empty());
    store.tokens.lock().unwrap().push(DeviceToken {
        token: "t1".to_string(),
        user_id: 1,
        device_name: "Mobile Device".to_string(),
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        last_used_at: None,
    });
    assert_eq!(DeviceService::get_user_devices_with(&store, 1).await.unwrap().len(), 1);
    assert!(DeviceService::revoke_device_with(&store, &notifier, 1, "t1").await.unwrap());
    assert!(!DeviceService::revoke_device_with(&store, &notifier, 1, "t1").await.unwrap());

    store.fail_next("db");
    assert_eq!(
        DeviceService::get_user_devices_with(&store, 1).await.unwrap_err().status,
        502
    );
}

#[tokio::test]
async fn revoke_notifies_live_device_only_on_success() {
    let (store, _client) = harness();
    let notifier = FakeNotifier::new();
    store.tokens.lock().unwrap().push(DeviceToken {
        token: "live-1".to_string(),
        user_id: 1,
        device_name: "Phone".to_string(),
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        last_used_at: None,
    });

    assert!(DeviceService::revoke_device_with(&store, &notifier, 1, "live-1").await.unwrap());
    assert_eq!(notifier.calls.lock().unwrap().as_slice(), ["live-1"]);

    // Revoking nothing notifies nothing.
    assert!(!DeviceService::revoke_device_with(&store, &notifier, 1, "gone").await.unwrap());
    assert_eq!(notifier.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn revoke_succeeds_when_push_fails() {
    let (store, _client) = harness();
    let notifier = FakeNotifier::new();
    store.tokens.lock().unwrap().push(DeviceToken {
        token: "t9".to_string(),
        user_id: 1,
        device_name: "Phone".to_string(),
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        last_used_at: None,
    });
    // Best-effort: the push is fire-and-forget; revocation always commits.
    assert!(DeviceService::revoke_device_with(&store, &notifier, 1, "t9").await.unwrap());
    assert!(DeviceService::get_user_devices_with(&store, 1).await.unwrap().is_empty());

    // Without a binding there is simply no one to notify.
    let quiet = NoNotify;
    store.tokens.lock().unwrap().push(DeviceToken {
        token: "t8".to_string(),
        user_id: 1,
        device_name: "Phone".to_string(),
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        last_used_at: None,
    });
    assert!(DeviceService::revoke_device_with(&store, &quiet, 1, "t8").await.unwrap());
}

#[tokio::test]
async fn authorize_and_notify_hub() {
    let (store, client) = harness();
    let hub = FakeHubNotifier::new();
    let sid = valid_session_id();
    client.seed(&sid, pending_state("future"));

    let res = DeviceService::authorize_device_and_notify_with(
        &store,
        &client,
        &hub,
        &sample_user(7),
        &sid,
        Some("My Phone"),
        || Ok("tok-generated".to_string()),
    )
    .await
    .unwrap();

    assert_eq!(res.status, "authorized");
    // Verify Hub was notified of authorization
    let calls = hub.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (7, DeviceLifecycleKind::Authorized, None));

    // When authorization fails, Hub should NOT be notified
    let hub2 = FakeHubNotifier::new();
    let fail_res = DeviceService::authorize_device_and_notify_with(
        &store,
        &client,
        &hub2,
        &sample_user(7),
        "nonexistent-session",
        None,
        || Ok("tok".to_string()),
    )
    .await;
    assert!(fail_res.is_err());
    assert_eq!(hub2.calls.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn revoke_and_notify_hub() {
    let (store, _client) = harness();
    let dev_notifier = FakeNotifier::new();
    let hub_notifier = FakeHubNotifier::new();

    store.tokens.lock().unwrap().push(DeviceToken {
        token: "live-token-1".to_string(),
        user_id: 42,
        device_name: "Phone".to_string(),
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        last_used_at: None,
    });

    // 1. Web dashboard revocation
    let revoked = DeviceService::revoke_device_and_notify_with(
        &store,
        &dev_notifier,
        &hub_notifier,
        42,
        "live-token-1",
        DeviceLifecycleKind::Revoked,
    )
    .await
    .unwrap();

    assert!(revoked);
    assert_eq!(dev_notifier.calls.lock().unwrap().as_slice(), ["live-token-1"]);
    let hub_calls = hub_notifier.calls.lock().unwrap().clone();
    assert_eq!(hub_calls.len(), 1);
    assert_eq!(
        hub_calls[0],
        (42, DeviceLifecycleKind::Revoked, Some("live-token-1".to_string()))
    );

    // 2. Mobile self-disconnect
    store.tokens.lock().unwrap().push(DeviceToken {
        token: "live-token-2".to_string(),
        user_id: 42,
        device_name: "Phone 2".to_string(),
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        last_used_at: None,
    });

    let disconnected = DeviceService::revoke_device_and_notify_with(
        &store,
        &dev_notifier,
        &hub_notifier,
        42,
        "live-token-2",
        DeviceLifecycleKind::Disconnected,
    )
    .await
    .unwrap();

    assert!(disconnected);
    assert_eq!(
        dev_notifier.calls.lock().unwrap().as_slice(),
        ["live-token-1", "live-token-2"]
    );
    let hub_calls = hub_notifier.calls.lock().unwrap().clone();
    assert_eq!(hub_calls.len(), 2);
    assert_eq!(
        hub_calls[1],
        (42, DeviceLifecycleKind::Disconnected, Some("live-token-2".to_string()))
    );

    // 3. Revoking nonexistent token does not notify
    let gone = DeviceService::revoke_device_and_notify_with(
        &store,
        &dev_notifier,
        &hub_notifier,
        42,
        "nonexistent",
        DeviceLifecycleKind::Revoked,
    )
    .await
    .unwrap();

    assert!(!gone);
    assert_eq!(dev_notifier.calls.lock().unwrap().len(), 2);
    assert_eq!(hub_notifier.calls.lock().unwrap().len(), 2);
}

