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

pub fn upsert_device_session_sql() -> String {
    format!(
        "INSERT INTO device_sessions (session_id, short_code, status, expires_at, created_at, updated_at) \
         VALUES (?1, ?2, 'pending', {SESSION_EXPIRY_SQL}, {SQL_NOW}, {SQL_NOW}) \
         ON CONFLICT(session_id) DO UPDATE SET short_code = excluded.short_code, status = 'pending', \
         expires_at = {SESSION_EXPIRY_SQL}, updated_at = {SQL_NOW}"
    )
}

pub fn get_device_session_sql() -> String {
    format!(
        "SELECT session_id, short_code, \
                CASE \
                    WHEN status = 'pending' AND expires_at <= {SQL_NOW} THEN 'expired' \
                    ELSE status \
                END as status, \
                user_id, device_token, created_at, expires_at, updated_at \
         FROM device_sessions WHERE session_id = ?1 LIMIT 1"
    )
}

pub fn find_pending_session_sql() -> String {
    format!(
        "SELECT session_id, short_code, status, user_id, device_token, created_at, expires_at, updated_at \
         FROM device_sessions \
         WHERE (session_id = ?1 OR lower(short_code) = ?2) \
           AND status = 'pending' \
           AND expires_at > {SQL_NOW} \
         ORDER BY created_at DESC LIMIT 1"
    )
}

pub fn insert_device_token_sql() -> String {
    format!(
        "INSERT INTO device_tokens (token, user_id, device_name, expires_at, created_at, last_used_at) \
         VALUES (?1, ?2, ?3, {TOKEN_EXPIRY_SQL}, {SQL_NOW}, {SQL_NOW})"
    )
}

pub fn mark_session_authorized_sql() -> String {
    format!(
        "UPDATE device_sessions SET status = 'authorized', user_id = ?1, device_token = ?2, updated_at = {SQL_NOW} \
         WHERE session_id = ?3"
    )
}

pub fn get_user_devices_sql() -> String {
    format!(
        "SELECT token, user_id, device_name, created_at, expires_at, last_used_at \
         FROM device_tokens WHERE user_id = ?1 AND expires_at > {SQL_NOW} \
         ORDER BY created_at DESC"
    )
}

/// Lowercase lookup key used by `find_pending_session`.
pub fn normalize_session_lookup_key(code_or_session_id: &str) -> String {
    code_or_session_id.to_lowercase()
}
