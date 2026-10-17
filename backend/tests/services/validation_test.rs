use backend::services::validation::{
    derive_short_code, normalize_code_or_session, normalize_device_name,
    normalize_session_lookup, present_short_code, validate_session_id,
};

#[test]
fn validate_session_id_accepts_32_hex_and_trims() {
    let raw = format!("  {}  ", "d".repeat(32));
    let got = validate_session_id(&raw).unwrap();
    assert_eq!(got, "d".repeat(32));
}

#[test]
fn validate_session_id_rejects() {
    assert!(validate_session_id("").is_err());
    assert!(validate_session_id("   ").is_err());
    assert!(validate_session_id(&"a".repeat(31)).is_err());
    assert!(validate_session_id(&"z".repeat(32)).is_err());
    let err = validate_session_id("short").unwrap_err();
    assert_eq!(err.status, 400);
}

#[test]
fn derive_and_present_short_code() {
    let id = "AbC123".to_string() + &"d".repeat(26);
    assert_eq!(derive_short_code(&id), "abc123");
    assert_eq!(present_short_code("abc123"), "ABC123");
}

#[test]
fn normalize_code_or_session_ok_and_err() {
    assert_eq!(
        normalize_code_or_session("  ABC123 ").unwrap(),
        "ABC123".to_string()
    );
    let err = normalize_code_or_session("   ").unwrap_err();
    assert_eq!(err.status, 400);
    assert!(normalize_code_or_session("").is_err());
}

#[test]
fn normalize_session_lookup_ok_and_err() {
    assert_eq!(
        normalize_session_lookup(" sess ").unwrap(),
        "sess".to_string()
    );
    assert_eq!(normalize_session_lookup("x").unwrap(), "x");
    let err = normalize_session_lookup("  ").unwrap_err();
    assert_eq!(err.status, 400);
}

#[test]
fn normalize_device_name_defaults_and_trims() {
    assert_eq!(normalize_device_name(None), "Mobile Device");
    assert_eq!(normalize_device_name(Some("")), "Mobile Device");
    assert_eq!(normalize_device_name(Some("   ")), "Mobile Device");
    assert_eq!(normalize_device_name(Some("  Pixel  ")), "Pixel");
}
