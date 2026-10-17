use backend::common::DeviceLifecycleKind;
use backend::routes::validation::{
    require_bearer_header, require_check_session_id, require_credential,
    require_route_sub, require_route_token, require_session_id, require_sub_match,
    resolve_code_or_session, resolve_lifecycle_kind,
};
use backend::models::auth::GoogleAuthRequest;
use backend::models::device::{AuthorizeDeviceRequest, CheckSessionRequest, CreateSessionRequest};

#[test]
fn require_credential_ok_and_missing() {
    let b = GoogleAuthRequest { credential: Some("  tok  ".to_string()) };
    assert_eq!(require_credential(&b).unwrap(), "tok");
    let b = GoogleAuthRequest { credential: None };
    assert_eq!(require_credential(&b).unwrap_err().status, 400);
    let b = GoogleAuthRequest { credential: Some("   ".to_string()) };
    assert!(require_credential(&b).is_err());
}

#[test]
fn require_session_id_ok_and_missing() {
    let b = CreateSessionRequest { session_id: Some(" sid ".to_string()) };
    assert_eq!(require_session_id(&b).unwrap(), "sid");
    let b = CreateSessionRequest { session_id: None };
    assert_eq!(require_session_id(&b).unwrap_err().status, 400);
    let b = CreateSessionRequest { session_id: Some("".to_string()) };
    assert!(require_session_id(&b).is_err());
}

#[test]
fn resolve_code_or_session_prefers_session_id() {
    let b = AuthorizeDeviceRequest {
        session_id: Some("sess".to_string()),
        code: Some("code".to_string()),
        device_name: None,
    };
    assert_eq!(resolve_code_or_session(&b).unwrap(), "sess");
    let b = AuthorizeDeviceRequest {
        session_id: None,
        code: Some("  CODE1 ".to_string()),
        device_name: None,
    };
    assert_eq!(resolve_code_or_session(&b).unwrap(), "CODE1");
    let b = AuthorizeDeviceRequest {
        session_id: None,
        code: None,
        device_name: None,
    };
    assert_eq!(resolve_code_or_session(&b).unwrap_err().status, 400);
    let b = AuthorizeDeviceRequest {
        session_id: Some("  ".to_string()),
        code: Some(" ".to_string()),
        device_name: None,
    };
    assert!(resolve_code_or_session(&b).is_err());
}

#[test]
fn require_check_session_id_ok_and_missing() {
    let b = CheckSessionRequest { session_id: Some(" s ".to_string()) };
    assert_eq!(require_check_session_id(&b).unwrap(), "s");
    let b = CheckSessionRequest { session_id: None };
    assert_eq!(require_check_session_id(&b).unwrap_err().status, 400);
}

#[test]
fn require_bearer_header_ok_and_err() {
    assert_eq!(
        require_bearer_header(Some("Bearer tok")).unwrap(),
        "tok"
    );
    assert_eq!(require_bearer_header(None).unwrap_err().status, 401);
    assert_eq!(
        require_bearer_header(Some("Basic x")).unwrap_err().status,
        401
    );
    assert!(require_bearer_header(Some("Bearer ")).is_err());
}

#[test]
fn require_route_token_ok_and_err() {
    assert_eq!(require_route_token(Some("t")).unwrap(), "t");
    assert_eq!(require_route_token(None).unwrap_err().status, 400);
    assert_eq!(require_route_token(Some("")).unwrap_err().status, 400);
}

#[test]
fn require_sub_match_and_route_sub() {
    assert!(require_sub_match("a", "a").is_ok());
    let err = require_sub_match("a", "b").unwrap_err();
    assert_eq!(err.status, 403);
    assert_eq!(require_route_sub(Some("s")).unwrap(), "s");
    assert_eq!(require_route_sub(None).unwrap_err().status, 400);
    assert_eq!(require_route_sub(Some("")).unwrap_err().status, 400);
}

#[test]
fn resolve_lifecycle_kind_distinguishes_self_disconnect() {
    // Caller's token matches token being revoked -> self-disconnect
    assert_eq!(
        resolve_lifecycle_kind("device-token-123", "device-token-123"),
        DeviceLifecycleKind::Disconnected
    );

    // Caller is a different token (e.g. web session JWT or another device) -> revocation
    assert_eq!(
        resolve_lifecycle_kind("web-jwt-session", "device-token-123"),
        DeviceLifecycleKind::Revoked
    );
    assert_eq!(
        resolve_lifecycle_kind("device-token-A", "device-token-B"),
        DeviceLifecycleKind::Revoked
    );
}

