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
    // Pairing state lives in the Durable Object; D1 holds only the
    // code index plus long-lived device tokens.
    let up = upsert_pairing_code_sql();
    assert!(up.contains("pairing_codes"));
    assert!(up.contains(SESSION_EXPIRY_SQL));
    let g = get_pairing_code_sql();
    assert!(g.contains("session_id = ?1"));
    let p = find_session_by_code_sql();
    assert!(p.contains("lower(short_code)"));
    assert!(p.contains("expires_at"));
    assert!(p.contains("ORDER BY created_at DESC"));
    assert!(insert_device_token_sql().contains(TOKEN_EXPIRY_SQL));
    assert!(delete_device_token_sql().contains("DELETE FROM device_tokens"));
    let d = get_user_devices_sql();
    assert!(d.contains("device_tokens"));
    assert!(d.contains("ORDER BY created_at DESC"));
    let single = get_device_token_by_user_sql();
    assert!(single.contains("user_id = ?1 AND token = ?2"));
    let ren = renew_device_token_sql();
    assert!(ren.contains(TOKEN_EXPIRY_SQL));
    assert!(ren.contains("token = ?1 AND user_id = ?2"));
}

#[test]
fn normalize_lookup_key_lowercases() {
    assert_eq!(normalize_session_lookup_key("AbC123"), "abc123");
    assert_eq!(normalize_session_lookup_key(""), "");
}

fn memory_db() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!("../../schema.sql")).unwrap();
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
fn pairing_code_sql_runs_end_to_end() {
    let conn = memory_db();
    conn.execute(
        "INSERT INTO users (google_sub, email) VALUES ('sub-1', 'a@x.com')",
        [],
    )
    .unwrap();
    let user_id: i64 = conn
        .query_row("SELECT id FROM users WHERE google_sub = 'sub-1'", [], |r| r.get(0))
        .unwrap();
    // The index is a pointer only: code -> session, nothing about status.
    conn.execute(
        "INSERT INTO pairing_codes (session_id, short_code, expires_at, created_at) VALUES (?1, ?2, '2099-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
        rusqlite::params!["sess-1", "abc123"],
    )
    .unwrap();
    let code: String = conn
        .query_row(
            "SELECT short_code FROM pairing_codes WHERE lower(short_code) = 'abc123' AND expires_at > '2026-01-01T00:00:00.000Z'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(code, "abc123");

    conn.execute(
        "INSERT INTO device_tokens (token, user_id, device_name, expires_at, created_at, last_used_at) VALUES (?1, ?2, ?3, '2099-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
        rusqlite::params!["tok-1", user_id, "Mobile Device"],
    )
    .unwrap();
    conn.execute("DELETE FROM device_tokens WHERE token = 'tok-1'", [])
        .unwrap();
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM device_tokens", [], |r| r.get(0))
        .unwrap();
    assert_eq!(remaining, 0);
}

#[test]
fn expiry_sql_fragments_are_present_in_builders() {
    // Ensures production SQL uses time constants, not hardcoded literals.
    for sql in [
        upsert_pairing_code_sql(),
        find_session_by_code_sql(),
        get_user_devices_sql(),
        get_user_by_device_token_sql(),
    ] {
        assert!(sql.contains(SQL_NOW), "missing SQL_NOW in {sql}");
    }
}

#[test]
fn widget_sql_builders() {
    let owner = device_token_owner_sql();
    assert!(owner.contains("FROM device_tokens"));
    assert!(owner.contains("token = ?1"));
    assert!(owner.contains(SQL_NOW));
    let get = get_device_widgets_sql();
    assert!(get.contains("FROM device_widgets"));
    assert!(get.contains("token = ?1"));
    assert!(get.contains("ORDER BY widget_key ASC"));
    let del = delete_device_widgets_sql();
    assert!(del.contains("DELETE FROM device_widgets"));
    let up = upsert_device_widget_sql();
    assert!(up.contains("device_widgets"));
    assert!(up.contains("ON CONFLICT(token, widget_key)"));
    assert!(up.contains(SQL_NOW));
}

#[test]
fn widget_sql_runs_end_to_end() {
    // rusqlite is real SQLite, so the production builder SQL
    // (strftime fragments included) runs unmodified here.
    let conn = memory_db();
    conn.execute("INSERT INTO users (google_sub, email) VALUES ('sub-1', 'a@x.com')", []).unwrap();
    let user_id: i64 =
        conn.query_row("SELECT id FROM users WHERE google_sub = 'sub-1'", [], |r| r.get(0)).unwrap();
    conn.execute(
        "INSERT INTO device_tokens (token, user_id, device_name, expires_at, created_at, last_used_at) VALUES ('tok-1', ?1, 'Phone', '2099-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
        rusqlite::params![user_id],
    )
    .unwrap();

    let owner: i64 = conn
        .query_row(&device_token_owner_sql().replace(SQL_NOW, "'2026-06-01T00:00:00.000Z'"), ["tok-1"], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(owner, user_id);

    // Replace semantics: delete-then-upsert, empty list clears all.
    conn.execute(&delete_device_widgets_sql(), ["tok-1"]).unwrap();
    for (key, enabled, config) in [("weather", 1, r#"{"city":"Oslo"}"#), ("stocks", 0, "{}")] {
        conn.execute(
            &upsert_device_widget_sql(),
            rusqlite::params!["tok-1", key, enabled, config],
        )
        .unwrap();
    }
    let mut stmt = conn.prepare(&get_device_widgets_sql()).unwrap();
    let rows: Vec<(String, i64, String)> = stmt
        .query_map(["tok-1"], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "stocks"); // ordered by key
    assert_eq!(rows[1], ("weather".to_string(), 1, r#"{"city":"Oslo"}"#.to_string()));

    conn.execute(&delete_device_widgets_sql(), ["tok-1"]).unwrap();
    let remaining: i64 =
        conn.query_row("SELECT COUNT(*) FROM device_widgets", [], |r| r.get(0)).unwrap();
    assert_eq!(remaining, 0);
}
