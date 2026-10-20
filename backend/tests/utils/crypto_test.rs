use backend::utils::{generate_secure_token, generate_secure_token_using, is_valid_session_id_hex};

#[test]
fn valid_session_id_hex_accepts_lower_upper_and_digits() {
    assert!(is_valid_session_id_hex(&"a".repeat(32)));
    assert!(is_valid_session_id_hex(&"A".repeat(32)));
    assert!(is_valid_session_id_hex(&"0123456789abcdefABCDEF".repeat(32 / 22 + 1)[..32]));
}

#[test]
fn valid_session_id_hex_rejects_wrong_length() {
    assert!(!is_valid_session_id_hex(""));
    assert!(!is_valid_session_id_hex(&"a".repeat(31)));
    assert!(!is_valid_session_id_hex(&"a".repeat(33)));
    assert!(!is_valid_session_id_hex(&"a".repeat(64)));
    assert!(!is_valid_session_id_hex(&"a".repeat(128)));
}

#[test]
fn valid_session_id_hex_rejects_non_hex() {
    assert!(!is_valid_session_id_hex(&"g".repeat(32)));
    assert!(!is_valid_session_id_hex(&" ".repeat(32)));
    let mut s = "a".repeat(32);
    s.replace_range(10..11, "z");
    assert!(!is_valid_session_id_hex(&s));
}

#[test]
fn valid_session_id_hex_rejects_whitespace_padded() {
    // Caller must trim before validating; raw padded input is invalid.
    let padded = format!("  {}  ", "b".repeat(32));
    assert!(!is_valid_session_id_hex(&padded));
    assert!(is_valid_session_id_hex(padded.trim()));
}

#[test]
fn generate_secure_token_shape_is_64_hex() {
    let tok = generate_secure_token().expect("token");
    assert_eq!(tok.len(), 64);
    assert!(tok.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn generate_secure_token_unique() {
    let a = generate_secure_token().unwrap();
    let b = generate_secure_token().unwrap();
    assert_ne!(a, b);
}

fn failing_entropy(_: &mut [u8]) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNEXPECTED)
}

fn fixed_entropy(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    buf.fill(0xab);
    Ok(())
}

#[test]
fn generate_secure_token_using_error_and_custom() {
    let err = generate_secure_token_using(failing_entropy).unwrap_err();
    assert!(err.starts_with("Failed to generate random bytes:"));

    let custom = generate_secure_token_using(fixed_entropy).unwrap();
    assert_eq!(custom, "ab".repeat(32));
}
