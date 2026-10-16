use backend::utils::{
    is_origin_allowed, is_websocket_upgrade, parse_allowed_origins, parse_bearer_header,
};
use backend::common::DEFAULT_ORIGINS;

#[test]
fn parse_allowed_origins_empty_falls_back_to_defaults() {
    let got = parse_allowed_origins("");
    let want: Vec<String> = DEFAULT_ORIGINS.iter().map(|s| s.to_string()).collect();
    assert_eq!(got, want);
    let got_ws = parse_allowed_origins("  ,  , ");
    assert_eq!(got_ws, want);
}

#[test]
fn parse_allowed_origins_single_and_multi() {
    assert_eq!(
        parse_allowed_origins("https://a.example"),
        vec!["https://a.example".to_string()]
    );
    assert_eq!(
        parse_allowed_origins("https://a.example, https://b.example ,,"),
        vec![
            "https://a.example".to_string(),
            "https://b.example".to_string()
        ]
    );
}

#[test]
fn parse_allowed_origins_trims() {
    assert_eq!(
        parse_allowed_origins("  https://a.example  ,  https://b.example "),
        vec![
            "https://a.example".to_string(),
            "https://b.example".to_string()
        ]
    );
}

#[test]
fn is_origin_allowed_matches_exactly() {
    let allowed = vec!["https://a.example".to_string()];
    assert!(is_origin_allowed(Some("https://a.example"), &allowed));
    assert!(!is_origin_allowed(Some("https://b.example"), &allowed));
    assert!(!is_origin_allowed(None, &allowed));
    assert!(!is_origin_allowed(Some("https://a.example"), &[]));
}

#[test]
fn websocket_upgrade_bypasses_cors_mutation() {
    assert!(is_websocket_upgrade(101));
    assert!(!is_websocket_upgrade(200));
    assert!(!is_websocket_upgrade(204));
    assert!(!is_websocket_upgrade(500));
}

#[test]
fn parse_bearer_header_valid() {
    assert_eq!(
        parse_bearer_header(Some("Bearer abc123")),
        Some("abc123".to_string())
    );
}

#[test]
fn parse_bearer_header_rejects() {
    assert_eq!(parse_bearer_header(None), None);
    assert_eq!(parse_bearer_header(Some("")), None);
    assert_eq!(parse_bearer_header(Some("Bearer ")), None);
    assert_eq!(parse_bearer_header(Some("Basic abc")), None);
    assert_eq!(parse_bearer_header(Some("bearer abc")), None);
    assert_eq!(parse_bearer_header(Some("Bearer")), None);
}
