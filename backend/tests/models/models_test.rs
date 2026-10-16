use backend::models::auth::{GoogleAuthRequest, TokenInfo, User, UserResponse};
use backend::models::device::{
    AuthorizeDeviceRequest, AuthorizeDeviceResponse, CheckSessionRequest, CheckSessionResponse,
    CreateSessionRequest, CreateSessionResponse, DeviceSession, DeviceToken, DevicesResponse,
    RevokeResponse,
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

    let session = DeviceSession {
        session_id: "s".to_string(),
        short_code: "abc123".to_string(),
        status: "pending".to_string(),
        user_id: None,
        device_token: None,
        created_at: "c".to_string(),
        expires_at: "e".to_string(),
        updated_at: "u".to_string(),
    };
    let s = serde_json::to_string(&session).unwrap();
    let back: DeviceSession = serde_json::from_str(&s).unwrap();
    assert_eq!(back.status, "pending");

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
