use worker::*;

use super::queries;
use crate::models::device::{DeviceSession, DeviceToken};

pub async fn upsert_device_session(
    db: &D1Database,
    session_id: &str,
    short_code: &str,
) -> Result<DeviceSession> {
    let args = [D1Type::Text(session_id), D1Type::Text(short_code)];
    let stmt = db.prepare(queries::upsert_device_session_sql());
    stmt.bind_refs(&args)?.run().await?;

    get_device_session(db, session_id)
        .await?
        .ok_or_else(|| Error::RustError("Failed to load device session after insert.".to_string()))
}

pub async fn get_device_session(db: &D1Database, session_id: &str) -> Result<Option<DeviceSession>> {
    let stmt = db.prepare(queries::get_device_session_sql());
    let args = [D1Type::Text(session_id)];
    let stmt = stmt.bind_refs(&args)?;
    stmt.first::<DeviceSession>(None).await
}

pub async fn find_pending_session(
    db: &D1Database,
    code_or_session_id: &str,
) -> Result<Option<DeviceSession>> {
    let lower_code = queries::normalize_session_lookup_key(code_or_session_id);
    let stmt = db.prepare(queries::find_pending_session_sql());
    let args = [D1Type::Text(code_or_session_id), D1Type::Text(&lower_code)];
    let stmt = stmt.bind_refs(&args)?;
    stmt.first::<DeviceSession>(None).await
}

pub async fn authorize_device_session(
    db: &D1Database,
    session_id: &str,
    user_id: i64,
    device_token: &str,
    device_name: &str,
) -> Result<()> {
    // 1. Insert device token with 6-month validity
    let token_args = [
        D1Type::Text(device_token),
        D1Type::Integer(user_id as i32),
        D1Type::Text(device_name),
    ];
    let token_stmt = db.prepare(queries::insert_device_token_sql());
    token_stmt.bind_refs(&token_args)?.run().await?;

    // 2. Mark session as authorized
    let session_args = [
        D1Type::Integer(user_id as i32),
        D1Type::Text(device_token),
        D1Type::Text(session_id),
    ];
    let session_stmt = db.prepare(queries::mark_session_authorized_sql());
    session_stmt.bind_refs(&session_args)?.run().await?;

    Ok(())
}

pub async fn get_user_devices(db: &D1Database, user_id: i64) -> Result<Vec<DeviceToken>> {
    let stmt = db.prepare(queries::get_user_devices_sql());
    let args = [D1Type::Integer(user_id as i32)];
    let stmt = stmt.bind_refs(&args)?;
    let result = stmt.all().await?;
    result.results::<DeviceToken>()
}

pub async fn revoke_device_token(db: &D1Database, user_id: i64, token: &str) -> Result<bool> {
    let stmt = db.prepare(
        "DELETE FROM device_tokens WHERE user_id = ?1 AND token = ?2"
    );
    let args = [D1Type::Integer(user_id as i32), D1Type::Text(token)];
    let res = stmt.bind_refs(&args)?.run().await?;
    Ok(res.success())
}

