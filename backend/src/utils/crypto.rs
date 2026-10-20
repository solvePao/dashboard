use crate::common::SESSION_ID_HEX_LEN;

pub type EntropySource = fn(&mut [u8]) -> std::result::Result<(), getrandom::Error>;

pub fn generate_secure_token_using(
    get_entropy: EntropySource,
) -> std::result::Result<String, String> {
    let mut bytes = [0u8; 32];
    get_entropy(&mut bytes).map_err(|e| format!("Failed to generate random bytes: {e:?}"))?;
    let mut hex_str = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(hex_str, "{:02x}", b);
    }
    Ok(hex_str)
}

pub fn generate_secure_token() -> std::result::Result<String, String> {
    generate_secure_token_using(getrandom::getrandom)
}

pub fn is_valid_session_id_hex(s: &str) -> bool {
    s.len() == SESSION_ID_HEX_LEN && s.chars().all(|c| c.is_ascii_hexdigit())
}

