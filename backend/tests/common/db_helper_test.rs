use backend::common::{
    DeviceLifecycleKind, ALLOWED_HEADERS, ALLOWED_METHODS, CORS_MAX_AGE, DB_BINDING_NAME,
    DEFAULT_DEVICE_NAME, DEFAULT_ORIGINS, DEVICE_BINDING_NAME, ENV_ENVIRONMENT,
    ENV_FRONTEND_ORIGINS, FAILED_AUTHORIZE_DELAY_SECS, HUB_BINDING_NAME, PAIRING_BINDING_NAME,
    SECRET_GOOGLE_CLIENT_ID, SESSION_COOKIE_NAME, SESSION_EXPIRY_SQL, SESSION_ID_HEX_LEN,
    SESSION_TTL_SECS, SHORT_CODE_LEN, SQL_NOW,
};
use backend::db::helper::text_or_null;
use worker::D1Type;

#[test]
fn common_constants_match_contract() {
    assert_eq!(SESSION_ID_HEX_LEN, 32);
    assert_eq!(SHORT_CODE_LEN, 6);
    assert_eq!(DB_BINDING_NAME, "dashboard");
    assert_eq!(DEFAULT_DEVICE_NAME, "Mobile Device");
    assert_eq!(ALLOWED_METHODS, "GET, POST, DELETE, OPTIONS");
    assert_eq!(ALLOWED_HEADERS, "Content-Type, Authorization");
    assert_eq!(CORS_MAX_AGE, "86400");
    assert!(!SQL_NOW.is_empty());
    assert_eq!(DEFAULT_ORIGINS.len(), 4);
    assert_eq!(FAILED_AUTHORIZE_DELAY_SECS, 5);
    assert_eq!(PAIRING_BINDING_NAME, "PAIRING_OBJECT");
    assert_eq!(DEVICE_BINDING_NAME, "DEVICE_OBJECT");
    assert_eq!(HUB_BINDING_NAME, "DEVICES_HUB");
    assert_eq!(SESSION_COOKIE_NAME, "dashboard_session");
    assert_eq!(ENV_ENVIRONMENT, "ENVIRONMENT");
    assert_eq!(ENV_FRONTEND_ORIGINS, "FRONTEND_ORIGINS");
    assert_eq!(SECRET_GOOGLE_CLIENT_ID, "GOOGLE_CLIENT_ID");
    // Pairing window is 2 minutes on both clocks (D1 index and object alarm).
    assert_eq!(SESSION_TTL_SECS, 120);
    assert!(SESSION_EXPIRY_SQL.contains("+2 minutes"));
}

#[test]
fn device_lifecycle_kind_contract() {
    // as_str & Display
    assert_eq!(DeviceLifecycleKind::Authorized.as_str(), "authorized");
    assert_eq!(DeviceLifecycleKind::Revoked.as_str(), "revoked");
    assert_eq!(DeviceLifecycleKind::Disconnected.as_str(), "disconnected");

    assert_eq!(format!("{}", DeviceLifecycleKind::Authorized), "authorized");
    assert_eq!(format!("{}", DeviceLifecycleKind::Revoked), "revoked");
    assert_eq!(format!("{}", DeviceLifecycleKind::Disconnected), "disconnected");

    // Serde roundtrip (snake_case)
    let auth_json = serde_json::to_string(&DeviceLifecycleKind::Authorized).unwrap();
    assert_eq!(auth_json, "\"authorized\"");
    let rev_json = serde_json::to_string(&DeviceLifecycleKind::Revoked).unwrap();
    assert_eq!(rev_json, "\"revoked\"");
    let disc_json = serde_json::to_string(&DeviceLifecycleKind::Disconnected).unwrap();
    assert_eq!(disc_json, "\"disconnected\"");

    assert_eq!(
        serde_json::from_str::<DeviceLifecycleKind>("\"authorized\"").unwrap(),
        DeviceLifecycleKind::Authorized
    );
    assert_eq!(
        serde_json::from_str::<DeviceLifecycleKind>("\"revoked\"").unwrap(),
        DeviceLifecycleKind::Revoked
    );
    assert_eq!(
        serde_json::from_str::<DeviceLifecycleKind>("\"disconnected\"").unwrap(),
        DeviceLifecycleKind::Disconnected
    );

    // Rejection of invalid strings
    assert!(serde_json::from_str::<DeviceLifecycleKind>("\"Authorized\"").is_err());
    assert!(serde_json::from_str::<DeviceLifecycleKind>("\"unknown\"").is_err());
    assert!(serde_json::from_str::<DeviceLifecycleKind>("\"pending\"").is_err());
    assert!(serde_json::from_str::<DeviceLifecycleKind>("\"\"").is_err());

    // Debug, Copy, Clone, Eq
    let copy = DeviceLifecycleKind::Authorized;
    let clone = copy;
    assert_eq!(copy, clone);
    assert_ne!(DeviceLifecycleKind::Authorized, DeviceLifecycleKind::Revoked);
    assert_eq!(format!("{copy:?}"), "Authorized");
}

#[test]
fn user_id_param_binds_and_rejects_overflow() {
    match backend::db::helper::user_id_param(42) {
        Ok(D1Type::Integer(n)) => assert_eq!(n, 42),
        _ => panic!("expected integer"),
    }
    assert!(backend::db::helper::user_id_param(i64::from(i32::MAX) + 1).is_err());
}

#[test]
fn text_or_null_maps() {
    match text_or_null(Some("x")) {
        D1Type::Text(s) => assert_eq!(s, "x"),
        _ => panic!("expected text"),
    }
    match text_or_null(Some("")) {
        D1Type::Text(s) => assert_eq!(s, ""),
        _ => panic!("expected text"),
    }
    match text_or_null(None) {
        D1Type::Null => {},
        _ => panic!("expected null"),
    }
}
