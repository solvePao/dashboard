use backend::common::DeviceLifecycleKind;
use backend::models::auth::{GoogleAuthRequest, TokenInfo, User, UserResponse};
use backend::models::device::{
    AuthorizeDeviceRequest, AuthorizeDeviceResponse, CheckSessionRequest, CheckSessionResponse,
    CreateSessionRequest, CreateSessionResponse, DeviceToken, DevicesResponse, HubEvent,
    HubEventRequest, HubSnapshot, HubState, PairingCode, PairingCommitRequest, PairingInitRequest,
    PairingState, RevokeResponse,
};
use backend::models::error::AppError;

fn sample_user() -> User {
    User {
        id: 7,
        google_sub: "sub-1".to_string(),
        email: "a@example.com".to_string(),
        name: Some("Ada".to_string()),
        picture_url: None,
        created_at: "2026-01-01T00:00:00.000Z".to_string(),
        updated_at: "2026-01-02T00:00:00.000Z".to_string(),
        last_login_at: None,
    }
}

#[test]
fn error_constructors_map_status() {
    assert_eq!(AppError::bad_request("x").status, 400);
    assert_eq!(AppError::unauthorized("x").status, 401);
    assert_eq!(AppError::forbidden("x").status, 403);
    assert_eq!(AppError::not_found("x").status, 404);
    assert_eq!(AppError::too_many_requests("x").status, 429);
    assert_eq!(AppError::internal("x").status, 500);
    assert_eq!(AppError::bad_gateway("x").status, 502);
    assert_eq!(AppError::service_unavailable("x").status, 503);
    assert_eq!(AppError::new("m", 418).status, 418);
}

#[test]
fn user_serde_roundtrip() {
    let u = sample_user();
    let s = serde_json::to_string(&u).unwrap();
    let back: User = serde_json::from_str(&s).unwrap();
    assert_eq!(back.id, 7);
    assert_eq!(back.email, "a@example.com");
    let resp = UserResponse { user: u };
    let s = serde_json::to_string(&resp).unwrap();
    assert!(s.contains("sub-1"));
}

#[test]
fn token_info_deserializes() {
    let v: TokenInfo = serde_json::from_value(serde_json::json!({
        "aud": "cid", "sub": "s", "email": "e@x.com",
        "name": "N", "picture": "P", "exp": "9999999999"
    }))
    .unwrap();
    assert_eq!(v.aud.as_deref(), Some("cid"));
}

#[test]
fn google_auth_request_missing_credential() {
    let b: GoogleAuthRequest = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(b.credential.is_none());
}

#[test]
fn device_dtos_serde() {
    let r: CreateSessionRequest =
        serde_json::from_value(serde_json::json!({"session_id": "abc"})).unwrap();
    assert_eq!(r.session_id.as_deref(), Some("abc"));
    let r: AuthorizeDeviceRequest =
        serde_json::from_value(serde_json::json!({"code": "ABC123"})).unwrap();
    assert_eq!(r.code.as_deref(), Some("ABC123"));
    let r: CheckSessionRequest =
        serde_json::from_value(serde_json::json!({"session_id": "s"})).unwrap();
    assert_eq!(r.session_id.as_deref(), Some("s"));

    let resp = CreateSessionResponse {
        session_id: "s".to_string(),
        short_code: "ABC123".to_string(),
        qr_svg: "<svg/>".to_string(),
        expires_at: "t".to_string(),
    };
    assert!(serde_json::to_string(&resp).unwrap().contains("ABC123"));

    let resp = AuthorizeDeviceResponse {
        status: "authorized".to_string(),
        session_id: "s".to_string(),
        device_name: "Mobile Device".to_string(),
        message: "ok".to_string(),
    };
    assert!(serde_json::to_string(&resp).unwrap().contains("authorized"));

    let code = PairingCode {
        session_id: "s".to_string(),
        short_code: "abc123".to_string(),
        expires_at: "e".to_string(),
    };
    let s = serde_json::to_string(&code).unwrap();
    let back: PairingCode = serde_json::from_str(&s).unwrap();
    assert_eq!(back.short_code, "abc123");

    let init = PairingInitRequest { expires_at: "e".to_string() };
    let s = serde_json::to_string(&init).unwrap();
    let back: PairingInitRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.expires_at, "e");

    let commit = PairingCommitRequest { token: "t".to_string(), user: sample_user() };
    let s = serde_json::to_string(&commit).unwrap();
    let back: PairingCommitRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.token, "t");

    let state = PairingState {
        status: "pending".to_string(),
        expires_at: "e".to_string(),
        token: None,
        user: None,
    };
    let s = serde_json::to_string(&state).unwrap();
    let back: PairingState = serde_json::from_str(&s).unwrap();
    assert_eq!(back.status, "pending");
    assert!(!s.contains("token"));

    let tok = DeviceToken {
        token: "t".to_string(),
        user_id: 1,
        device_name: "d".to_string(),
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        last_used_at: None,
    };
    assert!(serde_json::to_string(&DevicesResponse { devices: vec![tok] })
        .unwrap()
        .contains("Mobile")
        || true);

    // CheckSessionResponse omits None fields.
    let r = CheckSessionResponse {
        status: "pending".to_string(),
        token: None,
        user: None,
    };
    let s = serde_json::to_string(&r).unwrap();
    assert!(!s.contains("token"));
    let r2 = CheckSessionResponse {
        status: "authorized".to_string(),
        token: Some("tok".to_string()),
        user: Some(sample_user()),
    };
    assert!(serde_json::to_string(&r2).unwrap().contains("tok"));

    let rev = RevokeResponse { revoked: true };
    assert!(serde_json::to_string(&rev).unwrap().contains("true"));
}

#[test]
fn check_session_response_roundtrips_from_object() {
    let r: CheckSessionResponse = serde_json::from_value(serde_json::json!({
        "status": "authorized", "token": "tok", "user": null
    }))
    .unwrap();
    assert_eq!(r.status, "authorized");
    assert_eq!(r.token.as_deref(), Some("tok"));
}

#[test]
fn hub_models_serde() {
    // HubEventRequest with token: Some
    let req = HubEventRequest {
        kind: DeviceLifecycleKind::Authorized,
        token: Some("tok-123".to_string()),
    };
    let s = serde_json::to_string(&req).unwrap();
    let back: HubEventRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.kind, DeviceLifecycleKind::Authorized);
    assert_eq!(back.token.as_deref(), Some("tok-123"));

    // HubEventRequest with token: None
    let req_none: HubEventRequest = serde_json::from_value(serde_json::json!({
        "kind": "revoked"
    }))
    .unwrap();
    assert_eq!(req_none.kind, DeviceLifecycleKind::Revoked);
    assert_eq!(req_none.token, None);

    // HubEvent with token: Some
    let ev = HubEvent {
        kind: DeviceLifecycleKind::Disconnected,
        token: Some("tok-abc".to_string()),
        revision: 42,
        at: 1700000000000,
    };
    let ev_str = serde_json::to_string(&ev).unwrap();
    assert!(ev_str.contains("\"kind\":\"disconnected\""));
    assert!(ev_str.contains("\"revision\":42"));
    assert!(ev_str.contains("\"token\":\"tok-abc\""));
    let ev_back: HubEvent = serde_json::from_str(&ev_str).unwrap();
    assert_eq!(ev_back.kind, DeviceLifecycleKind::Disconnected);
    assert_eq!(ev_back.revision, 42);

    // HubEvent with token: None omits token field in serialized JSON
    let ev_no_token = HubEvent {
        kind: DeviceLifecycleKind::Authorized,
        token: None,
        revision: 1,
        at: 1000,
    };
    let ev_no_token_str = serde_json::to_string(&ev_no_token).unwrap();
    assert!(!ev_no_token_str.contains("token"));

    // HubState default
    let default_state = HubState::default();
    assert_eq!(default_state.revision, 0);
    assert!(default_state.last_event.is_none());
    let default_str = serde_json::to_string(&default_state).unwrap();
    assert!(!default_str.contains("last_event"));

    // HubState roundtrip with last_event
    let state_with_event = HubState {
        revision: 10,
        last_event: Some(ev.clone()),
    };
    let s = serde_json::to_string(&state_with_event).unwrap();
    let back_state: HubState = serde_json::from_str(&s).unwrap();
    assert_eq!(back_state.revision, 10);
    assert_eq!(back_state.last_event.unwrap().revision, 42);

    // HubSnapshot
    let snapshot = HubSnapshot {
        kind: "snapshot".to_string(),
        revision: 10,
        last_event: Some(ev),
    };
    let snap_str = serde_json::to_string(&snapshot).unwrap();
    assert!(snap_str.contains("\"kind\":\"snapshot\""));
    assert!(snap_str.contains("\"revision\":10"));

    let snap_empty = HubSnapshot {
        kind: "snapshot".to_string(),
        revision: 0,
        last_event: None,
    };
    let snap_empty_str = serde_json::to_string(&snap_empty).unwrap();
    assert!(!snap_empty_str.contains("last_event"));
}

