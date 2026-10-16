use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::common::{JWT_CLOCK_SKEW_SECS, JWT_EXPIRY_SECS, JWT_SECRET_NAME, MIN_JWT_SECRET_LEN};
use crate::models::auth::JwtClaims;
use crate::models::error::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

const HEADER_B64: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"; // {"alg":"HS256","typ":"JWT"}

fn b64(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

fn unb64(s: &str) -> std::result::Result<Vec<u8>, String> {
    URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| "Invalid session token.".to_string())
}

/// Mints an HS256 JWT for `sub`/`uid`. Pure and unit-testable.
/// `now_secs` is unix seconds; expiry is `now + JWT_EXPIRY_SECS`.
pub fn mint_jwt(
    sub: &str,
    uid: i64,
    secret: &str,
    now_secs: u64,
) -> std::result::Result<String, String> {
    if secret.is_empty() {
        return Err("Server auth is not configured.".to_string());
    }
    if sub.is_empty() {
        return Err("Invalid user.".to_string());
    }
    let claims = JwtClaims {
        sub: sub.to_string(),
        uid,
        exp: now_secs.saturating_add(JWT_EXPIRY_SECS),
        iat: now_secs,
        nbf: None,
    };
    let payload = b64(
        &serde_json::to_vec(&claims).map_err(|_| "Failed to encode token.".to_string())?,
    );
    let signing_input = format!("{HEADER_B64}.{payload}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| "Server auth is not configured.".to_string())?;
    mac.update(signing_input.as_bytes());
    Ok(format!("{signing_input}.{}", b64(&mac.finalize().into_bytes())))
}

/// Verifies signature and expiry. Returns the claims on success.
/// All failures intentionally share one message so callers do not
/// leak whether the token was malformed, tampered, or expired —
/// except expiry, which the frontend uses to trigger re-login.
pub fn verify_jwt(
    token: &str,
    secret: &str,
    now_secs: u64,
) -> std::result::Result<JwtClaims, String> {
    if secret.is_empty() {
        return Err("Server auth is not configured.".to_string());
    }
    let mut parts = token.split('.');
    let (header, payload, signature) =
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(h), Some(p), Some(s), None) => (h, p, s),
            _ => return Err("Invalid session token.".to_string()),
        };
    if header != HEADER_B64 {
        return Err("Invalid session token.".to_string());
    }
    let signing_input = format!("{header}.{payload}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| "Server auth is not configured.".to_string())?;
    mac.update(signing_input.as_bytes());
    let expected = mac.finalize().into_bytes();
    let actual = unb64(signature)?;
    if actual.len() != expected.len() {
        return Err("Invalid session token.".to_string());
    }
    let mut diff = 0u8;
    for (a, b) in actual.iter().zip(expected.iter()) {
        diff |= a ^ b;
    }
    if diff != 0 {
        return Err("Invalid session token.".to_string());
    }
    let claims: JwtClaims =
        serde_json::from_slice(&unb64(payload)?).map_err(|_| "Invalid session token.".to_string())?;
    if claims.sub.is_empty() {
        return Err("Invalid session token.".to_string());
    }
    if claims.exp <= now_secs {
        return Err("Session token has expired.".to_string());
    }
    // Reject tokens issued in the future (beyond skew) or carrying a future
    // not-before: our minter never produces them, so they indicate skew or
    // forgery. `iat` after `exp` is likewise malformed.
    if claims.iat > now_secs.saturating_add(JWT_CLOCK_SKEW_SECS) {
        return Err("Invalid session token.".to_string());
    }
    if claims.iat > claims.exp {
        return Err("Invalid session token.".to_string());
    }
    if let Some(nbf) = claims.nbf {
        if nbf > now_secs {
            return Err("Invalid session token.".to_string());
        }
    }
    Ok(claims)
}

/// Reads `JWT_SECRET`, rejecting a missing, empty, or trivially short secret
/// with 503 (fail closed). Callers must propagate this instead of falling
/// back to an unauthenticated path.
pub fn jwt_secret(env: &worker::Env) -> AppResult<String> {
    let secret = env
        .secret(JWT_SECRET_NAME)
        .map(|s| s.to_string())
        .map_err(|_| AppError::service_unavailable("Server auth is not configured."))?;
    if secret.len() < MIN_JWT_SECRET_LEN {
        return Err(AppError::service_unavailable(
            "Server auth is not configured.",
        ));
    }
    Ok(secret)
}
