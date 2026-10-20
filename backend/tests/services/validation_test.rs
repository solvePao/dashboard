use backend::models::device::WidgetSaveItem;
use backend::services::validation::{
    derive_short_code, normalize_code_or_session, normalize_device_name,
    normalize_session_lookup, normalize_widget_config, normalize_widget_key, normalize_widgets,
    present_short_code, validate_session_id,
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

fn widget_item(key: Option<&str>, enabled: Option<bool>, config: Option<serde_json::Value>) -> WidgetSaveItem {
    WidgetSaveItem {
        widget_key: key.map(str::to_string),
        enabled,
        config,
    }
}

#[test]
fn normalize_widget_key_accepts_shaped_keys() {
    assert_eq!(normalize_widget_key(Some("weather")).unwrap(), "weather");
    assert_eq!(normalize_widget_key(Some("  stocks_2 ")).unwrap(), "stocks_2");
}

#[test]
fn normalize_widget_key_rejects() {
    for bad in [None, Some(""), Some("   "), Some("Weather"), Some("has space"), Some("has-dash"), Some("a/b")] {
        let err = normalize_widget_key(bad).unwrap_err();
        assert_eq!(err.status, 400);
    }
    let long = "w".repeat(33);
    assert_eq!(normalize_widget_key(Some(&long)).unwrap_err().status, 400);
}

#[test]
fn normalize_widget_config_defaults_to_empty_object() {
    assert_eq!(normalize_widget_config(None).unwrap(), serde_json::json!({}));
    let obj = serde_json::json!({"city": "Berlin"});
    assert_eq!(normalize_widget_config(Some(&obj)).unwrap(), obj);
    for bad in [serde_json::json!([]), serde_json::json!("x"), serde_json::json!(3)] {
        assert_eq!(normalize_widget_config(Some(&bad)).unwrap_err().status, 400);
    }
}

#[test]
fn normalize_widgets_body_cases() {
    // Missing body is a 400; empty list is valid (clears all).
    assert_eq!(normalize_widgets(None).unwrap_err().status, 400);
    assert!(normalize_widgets(Some(&[])).unwrap().is_empty());

    let items = vec![widget_item(Some("weather"), None, Some(serde_json::json!({"city": "Oslo"})))];
    let got = normalize_widgets(Some(&items)).unwrap();
    assert_eq!(got.len(), 1);
    // `enabled` defaults to true when omitted.
    assert!(got[0].enabled);
    assert_eq!(got[0].config["city"], "Oslo");

    // Duplicates and bad keys fail the whole body.
    let dup = vec![
        widget_item(Some("weather"), Some(true), None),
        widget_item(Some("weather"), Some(false), None),
    ];
    assert_eq!(normalize_widgets(Some(&dup)).unwrap_err().status, 400);
    let bad = vec![widget_item(Some("Nope"), Some(true), None)];
    assert_eq!(normalize_widgets(Some(&bad)).unwrap_err().status, 400);

    // Invalid widget config fails with 400.
    let bad_config = vec![widget_item(
        Some("weather"),
        Some(true),
        Some(serde_json::json!("not-an-object")),
    )];
    assert_eq!(normalize_widgets(Some(&bad_config)).unwrap_err().status, 400);

    // Over maximum widgets limit fails with 400.
    let too_many: Vec<_> = (0..17)
        .map(|i| widget_item(Some(&format!("w{i}")), Some(true), None))
        .collect();
    let err = normalize_widgets(Some(&too_many)).unwrap_err();
    assert_eq!(err.status, 400);
    assert_eq!(err.message, "Too many widgets.");
}
