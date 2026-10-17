use backend::utils::{
    build_expired_session_cookie, build_session_cookie, is_origin_allowed, is_websocket_upgrade,
    parse_allowed_origins, parse_bearer_header, parse_cookie_header,
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

#[test]
fn parse_cookie_header_cases() {
    assert_eq!(parse_cookie_header(None, "session"), None);
    assert_eq!(parse_cookie_header(Some(""), "session"), None);
    assert_eq!(parse_cookie_header(Some("   "), "session"), None);

    // Single cookie
    assert_eq!(
        parse_cookie_header(Some("dashboard_session=tok123"), "dashboard_session"),
        Some("tok123".to_string())
    );

    // Multiple cookies with spaces
    let header = "theme=dark; dashboard_session=tok123; user_pref=abc";
    assert_eq!(
        parse_cookie_header(Some(header), "dashboard_session"),
        Some("tok123".to_string())
    );
    assert_eq!(
        parse_cookie_header(Some(header), "theme"),
        Some("dark".to_string())
    );
    assert_eq!(
        parse_cookie_header(Some(header), "user_pref"),
        Some("abc".to_string())
    );

    // Semicolon without space
    let compact = "theme=light;dashboard_session=tok456;lang=en";
    assert_eq!(
        parse_cookie_header(Some(compact), "dashboard_session"),
        Some("tok456".to_string())
    );

    // Extra whitespace around key and value
    let padded = "  theme = dark  ;  dashboard_session = tok789  ";
    assert_eq!(
        parse_cookie_header(Some(padded), "dashboard_session"),
        Some("tok789".to_string())
    );

    // Prefix collision
    let collision = "dashboard_session_id=bad; dashboard_session=good";
    assert_eq!(
        parse_cookie_header(Some(collision), "dashboard_session"),
        Some("good".to_string())
    );

    // Empty value or no equals sign
    let empty_val = "bad_cookie; dashboard_session=; next=1";
    assert_eq!(parse_cookie_header(Some(empty_val), "dashboard_session"), None);
    assert_eq!(parse_cookie_header(Some(empty_val), "bad_cookie"), None);

    // Value containing '='
    let eq_in_val = "dashboard_session=part1=part2=part3";
    assert_eq!(
        parse_cookie_header(Some(eq_in_val), "dashboard_session"),
        Some("part1=part2=part3".to_string())
    );

    // Nonexistent name
    assert_eq!(parse_cookie_header(Some("a=1; b=2"), "c"), None);
}

#[test]
fn build_session_cookie_cases() {
    let secure_cookie = build_session_cookie("sess", "token123", 3600, true);
    assert_eq!(
        secure_cookie,
        "sess=token123; Path=/api; HttpOnly; Secure; SameSite=None; Max-Age=3600"
    );

    let dev_cookie = build_session_cookie("sess", "token123", 3600, false);
    assert_eq!(
        dev_cookie,
        "sess=token123; Path=/api; HttpOnly; SameSite=Lax; Max-Age=3600"
    );
}

#[test]
fn build_expired_session_cookie_cases() {
    let secure_expired = build_expired_session_cookie("sess", true);
    assert_eq!(
        secure_expired,
        "sess=; Path=/api; HttpOnly; Secure; SameSite=None; Max-Age=0"
    );

    let dev_expired = build_expired_session_cookie("sess", false);
    assert_eq!(
        dev_expired,
        "sess=; Path=/api; HttpOnly; SameSite=Lax; Max-Age=0"
    );
}



