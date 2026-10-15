use crate::common::SESSION_ID_HEX_LEN;

pub fn generate_secure_token() -> std::result::Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| format!("Failed to generate random bytes: {e:?}"))?;
    let mut hex_str = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(hex_str, "{:02x}", b);
    }
    Ok(hex_str)
}

pub fn is_valid_128_hex(s: &str) -> bool {
    s.len() == SESSION_ID_HEX_LEN && s.chars().all(|c| c.is_ascii_hexdigit())
}

