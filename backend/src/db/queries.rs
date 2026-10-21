use crate::common::{SESSION_EXPIRY_SQL, SQL_NOW, TOKEN_EXPIRY_SQL, USER_COLUMNS};

/// Pure SQL builders mirroring `src/db/users.rs` and `src/db/devices.rs`.
/// Tested directly and against in-memory SQLite; production `db::*`
/// functions delegate to these builders so query text is covered.
pub fn get_user_by_sub_sql() -> String {
    format!("SELECT {USER_COLUMNS} FROM users WHERE google_sub = ?1 LIMIT 1")
}

pub fn get_user_by_id_sql() -> String {
    format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?1 LIMIT 1")
}

pub fn upsert_user_sql() -> String {
    format!(
        "INSERT INTO users (google_sub, email, name, picture_url, last_login_at) \
         VALUES (?1, ?2, ?3, ?4, {SQL_NOW}) \
         ON CONFLICT(google_sub) DO UPDATE SET email = excluded.email, name = excluded.name, \
         picture_url = excluded.picture_url, last_login_at = {SQL_NOW}, updated_at = {SQL_NOW}"
    )
}

pub fn get_user_by_device_token_sql() -> String {
    format!(
        "SELECT users.id, users.google_sub, users.email, users.name, users.picture_url, \
                users.created_at, users.updated_at, users.last_login_at \
         FROM users \
         INNER JOIN device_tokens ON users.id = device_tokens.user_id \
         WHERE device_tokens.token = ?1 AND device_tokens.expires_at > {SQL_NOW} \
         LIMIT 1"
    )
}

pub fn touch_device_token_sql() -> String {
    format!("UPDATE device_tokens SET last_used_at = {SQL_NOW} WHERE token = ?1")
}

/// Pointer insert only. The authorized guard lives in the service/DO:
/// re-creating an authorized pairing never reaches this statement.
///
/// `created_at` is intentionally reset on conflict: `find_session_by_code_sql`
/// orders by `created_at DESC` so a refreshed session always wins a short-code
/// collision over an older, potentially expired one.
pub fn upsert_pairing_code_sql() -> String {
    format!(
        "INSERT INTO pairing_codes (session_id, short_code, expires_at, created_at) \
         VALUES (?1, ?2, {SESSION_EXPIRY_SQL}, {SQL_NOW}) \
         ON CONFLICT(session_id) DO UPDATE SET short_code = excluded.short_code, \
         expires_at = {SESSION_EXPIRY_SQL}, created_at = {SQL_NOW}"
    )
}

pub fn get_pairing_code_sql() -> String {
    "SELECT session_id, short_code, expires_at FROM pairing_codes \
     WHERE session_id = ?1 LIMIT 1"
        .to_string()
}

/// Code -> session translation. Newest wins when prefixes collide,
/// matching the previous pending-lookup ordering.
pub fn find_session_by_code_sql() -> String {
    format!(
        "SELECT session_id, short_code, expires_at FROM pairing_codes \
         WHERE lower(short_code) = ?1 AND expires_at > {SQL_NOW} \
         ORDER BY created_at DESC LIMIT 1"
    )
}

pub fn insert_device_token_sql() -> String {
    format!(
        "INSERT INTO device_tokens (token, user_id, device_name, expires_at, created_at, last_used_at) \
         VALUES (?1, ?2, ?3, {TOKEN_EXPIRY_SQL}, {SQL_NOW}, {SQL_NOW})"
    )
}

pub fn delete_device_token_sql() -> String {
    "DELETE FROM device_tokens WHERE token = ?1".to_string()
}

pub fn get_user_devices_sql() -> String {
    format!(
        "SELECT token, user_id, device_name, created_at, expires_at, last_used_at \
         FROM device_tokens WHERE user_id = ?1 AND expires_at > {SQL_NOW} \
         ORDER BY created_at DESC"
    )
}

/// Owner of one device token, or no row when unknown/expired.
/// Guards every widget read/write so users only touch own devices.
pub fn device_token_owner_sql() -> String {
    format!("SELECT user_id FROM device_tokens WHERE token = ?1 AND expires_at > {SQL_NOW} LIMIT 1")
}

pub fn get_device_token_by_user_sql() -> String {
    format!(
        "SELECT token, user_id, device_name, created_at, expires_at, last_used_at \
         FROM device_tokens WHERE user_id = ?1 AND token = ?2 AND expires_at > {SQL_NOW} LIMIT 1"
    )
}

pub fn renew_device_token_sql() -> String {
    format!(
        "UPDATE device_tokens SET expires_at = {TOKEN_EXPIRY_SQL}, last_used_at = {SQL_NOW} \
         WHERE token = ?1 AND user_id = ?2"
    )
}

pub fn get_device_widgets_sql() -> String {
    "SELECT widget_key, enabled, config_json, updated_at FROM device_widgets \
     WHERE token = ?1 ORDER BY widget_key ASC"
        .to_string()
}

pub fn delete_device_widgets_sql() -> String {
    "DELETE FROM device_widgets WHERE token = ?1".to_string()
}

pub fn upsert_device_widget_sql() -> String {
    format!(
        "INSERT INTO device_widgets (token, widget_key, enabled, config_json, updated_at) \
         VALUES (?1, ?2, ?3, ?4, {SQL_NOW}) \
         ON CONFLICT(token, widget_key) DO UPDATE SET enabled = excluded.enabled, \
         config_json = excluded.config_json, updated_at = {SQL_NOW}"
    )
}

/// Lowercase lookup key used by the pairing-code index lookup.
pub fn normalize_session_lookup_key(code_or_session_id: &str) -> String {
    code_or_session_id.to_lowercase()
}

