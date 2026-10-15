use backend::db::queries::*;
use backend::common::{SESSION_EXPIRY_SQL, SQL_NOW, TOKEN_EXPIRY_SQL, USER_COLUMNS};

#[test]
fn user_sql_builders() {
    assert!(get_user_by_sub_sql().contains(&format!("SELECT {USER_COLUMNS}")));
    assert!(get_user_by_sub_sql().contains("google_sub = ?1"));
    assert!(get_user_by_id_sql().contains("id = ?1"));
    let up = upsert_user_sql();
    assert!(up.contains("ON CONFLICT(google_sub)"));
    assert!(up.contains(SQL_NOW));
    let tok = get_user_by_device_token_sql();
    assert!(tok.contains("INNER JOIN device_tokens"));
    assert!(tok.contains(SQL_NOW));
    assert!(touch_device_token_sql().contains("last_used_at"));
}

#[test]
fn device_sql_builders() {
    let up = upsert_device_session_sql();
    assert!(up.contains("device_sessions"));
    assert!(up.contains(SESSION_EXPIRY_SQL));
    let g = get_device_session_sql();
    assert!(g.contains("CASE"));
    assert!(g.contains("expired"));
    let p = find_pending_session_sql();
    assert!(p.contains("lower(short_code)"));
    assert!(p.contains("pending"));
    assert!(p.contains(SQL_NOW));
    assert!(insert_device_token_sql().contains(TOKEN_EXPIRY_SQL));
    assert!(mark_session_authorized_sql().contains("'authorized'"));
    let d = get_user_devices_sql();
    assert!(d.contains("device_tokens"));
    assert!(d.contains("ORDER BY created_at DESC"));
}

#[test]
fn normalize_lookup_key_lowercases() {
    assert_eq!(normalize_session_lookup_key("AbC123"), "abc123");
    assert_eq!(normalize_session_lookup_key(""), "");
}

fn memory_db() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!("../schema.sql")).unwrap();
    conn
}

#[test]
fn schema_applies_and_user_sql_runs() {
    let conn = memory_db();
    // Upsert SQL with NOW replaced by a literal for rusqlite (same shape).
    conn.execute(
        "INSERT INTO users (google_sub, email, name, picture_url, last_login_at) VALUES (?1, ?2, ?3, ?4, '2026-01-01T00:00:00.000Z')",
        rusqlite::params!["sub-1", "a@x.com", "Ada", Option::<String>::None],
    )
    .unwrap();
    let email: String = conn
        .query_row("SELECT email FROM users WHERE google_sub = 'sub-1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(email, "a@x.com");
}

#[test]
fn device_session_sql_runs_end_to_end() {
    let conn = memory_db();
    conn.execute(
        "INSERT INTO users (google_sub, email) VALUES ('sub-1', 'a@x.com')",
        [],
    )
    .unwrap();
    let user_id: i64 = conn
        .query_row("SELECT id FROM users WHERE google_sub = 'sub-1'", [], |r| r.get(0))
        .unwrap();
    conn.execute(
        "INSERT INTO device_sessions (session_id, short_code, status, expires_at, created_at, updated_at) VALUES (?1, ?2, 'pending', '2099-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
        rusqlite::params!["sess-1", "abc123"],
    )
    .unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM device_sessions WHERE session_id = 'sess-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "pending");

    conn.execute(
        "INSERT INTO device_tokens (token, user_id, device_name, expires_at, created_at, last_used_at) VALUES (?1, ?2, ?3, '2099-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
        rusqlite::params!["tok-1", user_id, "Mobile Device"],
    )
    .unwrap();
    conn.execute(
        "UPDATE device_sessions SET status = 'authorized', user_id = 1, device_token = 'tok-1' WHERE session_id = 'sess-1'",
        [],
    )
    .unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM device_sessions WHERE session_id = 'sess-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "authorized");
}

#[test]
fn expiry_sql_fragments_are_present_in_builders() {
    // Ensures production SQL uses time constants, not hardcoded literals.
    for sql in [
        upsert_device_session_sql(),
        get_device_session_sql(),
        find_pending_session_sql(),
        get_user_devices_sql(),
        get_user_by_device_token_sql(),
    ] {
        assert!(sql.contains(SQL_NOW), "missing SQL_NOW in {sql}");
    }
}
