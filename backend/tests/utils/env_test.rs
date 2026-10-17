use backend::utils::{
    parse_environment, parse_frontend_origins_value, parse_is_production, validate_google_client_id,
};

#[test]
fn parse_is_production_cases() {
    assert!(parse_is_production(Some("production")));
    assert!(parse_is_production(Some(" production ")));
    assert!(!parse_is_production(Some("dev")));
    assert!(!parse_is_production(Some("staging")));
    assert!(!parse_is_production(Some("")));
    assert!(!parse_is_production(None));
}

#[test]
fn parse_environment_cases() {
    assert_eq!(parse_environment(Some("production")), "production");
    assert_eq!(parse_environment(Some("staging")), "staging");
    assert_eq!(parse_environment(Some("  custom-env  ")), "custom-env");
    assert_eq!(parse_environment(Some("")), "dev");
    assert_eq!(parse_environment(Some("   ")), "dev");
    assert_eq!(parse_environment(None), "dev");
}

#[test]
fn parse_frontend_origins_value_cases() {
    assert_eq!(
        parse_frontend_origins_value(Some("https://a.com,https://b.com")),
        Some("https://a.com,https://b.com".to_string())
    );
    assert_eq!(
        parse_frontend_origins_value(Some("  https://a.com  ")),
        Some("https://a.com".to_string())
    );
    assert_eq!(parse_frontend_origins_value(Some("")), None);
    assert_eq!(parse_frontend_origins_value(Some("   ")), None);
    assert_eq!(parse_frontend_origins_value(None), None);
}

#[test]
fn validate_google_client_id_cases() {
    assert_eq!(
        validate_google_client_id(Some("client-id-123.apps.googleusercontent.com")).unwrap(),
        "client-id-123.apps.googleusercontent.com"
    );
    assert_eq!(
        validate_google_client_id(Some("  trimmed-id  ")).unwrap(),
        "trimmed-id"
    );

    let err_none = validate_google_client_id(None).unwrap_err();
    assert_eq!(err_none.status, 503);
    assert!(err_none.message.contains("Google login is not configured"));

    let err_empty = validate_google_client_id(Some("")).unwrap_err();
    assert_eq!(err_empty.status, 503);

    let err_ws = validate_google_client_id(Some("   ")).unwrap_err();
    assert_eq!(err_ws.status, 503);
}

