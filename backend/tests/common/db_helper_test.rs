use backend::common::{
    ALLOWED_HEADERS, ALLOWED_METHODS, CORS_MAX_AGE, DB_BINDING_NAME, DEFAULT_DEVICE_NAME,
    DEFAULT_ORIGINS, FAILED_AUTHORIZE_DELAY_SECS, SESSION_ID_HEX_LEN, SHORT_CODE_LEN, SQL_NOW,
};
use backend::db::helper::text_or_null;
use worker::D1Type;

#[test]
fn common_constants_match_contract() {
    assert_eq!(SESSION_ID_HEX_LEN, 128);
    assert_eq!(SHORT_CODE_LEN, 6);
    assert_eq!(DB_BINDING_NAME, "dashboard");
    assert_eq!(DEFAULT_DEVICE_NAME, "Mobile Device");
    assert_eq!(ALLOWED_METHODS, "GET, POST, DELETE, OPTIONS");
    assert_eq!(ALLOWED_HEADERS, "Content-Type, Authorization");
    assert_eq!(CORS_MAX_AGE, "86400");
    assert!(!SQL_NOW.is_empty());
    assert_eq!(DEFAULT_ORIGINS.len(), 4);
    assert_eq!(FAILED_AUTHORIZE_DELAY_SECS, 5);
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
