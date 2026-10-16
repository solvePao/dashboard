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
fn jwt_rejects_wrong_header_and_short_signature() {
    let token = mint_jwt("sub-1", 7, SECRET, NOW).unwrap();
    let parts: Vec<&str> = token.split('.').collect();
    // Three parts but a foreign header: rejected before signature checks.
    assert!(verify_jwt("e30.e30.e30", SECRET, NOW).is_err());
    // Valid header, undecodable-length signature.
    let short_sig = format!("{}.{}.{}", parts[0], parts[1], "AA");
    assert!(verify_jwt(&short_sig, SECRET, NOW).is_err());
}

#[test]
fn jwt_rejects_malformed_claims_and_skewed_times() {
    // Empty subject with a valid signature: rejected, not anonymous.
    let no_sub = sign_claims("", 7, NOW + 3600, NOW, None);
    assert!(verify_jwt(&no_sub, SECRET, NOW).is_err());
    // Issued after expiry is malformed (iat within skew, past exp).
    let inverted = sign_claims("sub-1", 7, NOW + 30, NOW + 50, None);
    assert!(verify_jwt(&inverted, SECRET, NOW).is_err());
    // A future not-before is rejected; past is accepted.
    let future_nbf = sign_claims("sub-1", 7, NOW + 3600, NOW, Some(NOW + 600));
    assert!(verify_jwt(&future_nbf, SECRET, NOW).is_err());
    let past_nbf = sign_claims("sub-1", 7, NOW + 3600, NOW, Some(NOW - 10));
    assert!(verify_jwt(&past_nbf, SECRET, NOW).is_ok());
}

/// Signs arbitrary claims with the test secret (mirrors `mint_jwt`).
fn sign_claims(sub: &str, uid: i64, exp: u64, iat: u64, nbf: Option<u64>) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use backend::models::auth::JwtClaims;
    let header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&JwtClaims { sub: sub.to_string(), uid, exp, iat, nbf }).unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
    use hmac::Mac;
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
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
