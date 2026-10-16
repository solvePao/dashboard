use backend::common::JWT_EXPIRY_SECS;
use backend::utils::{mint_jwt, verify_jwt};

const SECRET: &str = "unit-test-jwt-secret-32chars-long!";
const NOW: u64 = 1_700_000_000;

#[test]
fn jwt_roundtrip_preserves_identity() {
    let token = mint_jwt("sub-1", 7, SECRET, NOW).unwrap();
    assert_eq!(token.split('.').count(), 3);
    let claims = verify_jwt(&token, SECRET, NOW).unwrap();
    assert_eq!(claims.sub, "sub-1");
    assert_eq!(claims.uid, 7);
    assert_eq!(claims.exp, NOW + JWT_EXPIRY_SECS);
    assert_eq!(claims.iat, NOW);
}

#[test]
fn jwt_rejects_tampered_and_malformed() {
    let token = mint_jwt("sub-1", 7, SECRET, NOW).unwrap();
    // Flip the last signature character.
    let mut tampered = token.clone();
    tampered.pop();
    tampered.push(if token.ends_with('A') { 'B' } else { 'A' });
    assert!(verify_jwt(&tampered, SECRET, NOW).is_err());

    for bad in ["", "a.b", "a.b.c.d", "not-a-token", &token[..token.len() / 2]] {
        assert!(verify_jwt(bad, SECRET, NOW).is_err(), "accepted {bad:?}");
    }
}

#[test]
fn jwt_rejects_wrong_secret_and_expiry() {
    let token = mint_jwt("sub-1", 7, SECRET, NOW).unwrap();
    assert!(verify_jwt(&token, "other-secret", NOW).is_err());
    // Exactly at expiry is expired.
    assert!(verify_jwt(&token, SECRET, NOW + JWT_EXPIRY_SECS).is_err());
    assert!(verify_jwt(&token, SECRET, NOW + JWT_EXPIRY_SECS - 1).is_ok());
}

#[test]
fn jwt_rejects_missing_secret_and_empty_sub() {
    assert!(mint_jwt("sub-1", 7, "", NOW).is_err());
    assert!(mint_jwt("", 7, SECRET, NOW).is_err());
    let token = mint_jwt("sub-1", 7, SECRET, NOW).unwrap();
    assert!(verify_jwt(&token, "", NOW).is_err());
}

#[test]
fn jwt_rejects_future_iat() {
    // Mint with a future `now`: signature is valid but `iat` lies ahead of
    // the verifier clock beyond skew, so verification must reject.
    let future = mint_jwt("sub-1", 7, SECRET, NOW + 600).unwrap();
    assert!(verify_jwt(&future, SECRET, NOW).is_err());
    // Within skew still verifies.
    let slight = mint_jwt("sub-1", 7, SECRET, NOW + 30).unwrap();
    assert!(verify_jwt(&slight, SECRET, NOW).is_ok());
}
