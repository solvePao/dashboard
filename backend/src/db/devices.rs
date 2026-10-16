use worker::*;

use super::helper::user_id_param;
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

pub async fn find_session_for_authorize(
    db: &D1Database,
    code_or_session_id: &str,
) -> Result<Option<DeviceSession>> {
    let lower_code = queries::normalize_session_lookup_key(code_or_session_id);
    let stmt = db.prepare(queries::find_session_for_authorize_sql());
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
) -> Result<bool> {
    // One batch = one transaction: D1 rolls back the whole sequence when
    // any statement fails. The session UPDATE is guarded on
    // status='pending'; a concurrent winner leaves 0 changed rows for the
    // loser, which must not overwrite user_id/device_token.
    let token_stmt = db
        .prepare(queries::insert_device_token_sql())
        .bind_refs(&[
            D1Type::Text(device_token),
            user_id_param(user_id)?,
            D1Type::Text(device_name),
        ])?;
    let session_stmt = db
        .prepare(queries::mark_session_authorized_sql())
        .bind_refs(&[
            user_id_param(user_id)?,
            D1Type::Text(device_token),
            D1Type::Text(session_id),
        ])?;
    let results = db.batch(vec![token_stmt, session_stmt]).await?;
    let changed = results
        .get(1)
        .and_then(|r| r.meta().ok().flatten())
        .and_then(|m| m.changes)
        .unwrap_or(0);
    if changed == 0 {
        // Compensate the token insert: the session was already
        // authorized/expired, so the token would otherwise be orphaned.
        let stmt = db.prepare("DELETE FROM device_tokens WHERE token = ?1");
        if let Ok(bound) = stmt.bind_refs(&[D1Type::Text(device_token)]) {
            let _ = bound.run().await;
        }
        return Ok(false);
    }

    Ok(true)
}

pub async fn get_user_devices(db: &D1Database, user_id: i64) -> Result<Vec<DeviceToken>> {
    let stmt = db.prepare(queries::get_user_devices_sql());
    let args = [user_id_param(user_id)?];
    let stmt = stmt.bind_refs(&args)?;
    let result = stmt.all().await?;
    result.results::<DeviceToken>()
}

pub async fn revoke_device_token(db: &D1Database, user_id: i64, token: &str) -> Result<bool> {
    let stmt = db.prepare(
        "DELETE FROM device_tokens WHERE user_id = ?1 AND token = ?2"
    );
    let args = [user_id_param(user_id)?, D1Type::Text(token)];
    let res = stmt.bind_refs(&args)?.run().await?;
    // `success()` is true even when zero rows matched, so the row count
    // is the only honest signal for "revoked".
    let changed = res.meta()?.and_then(|m| m.changes).unwrap_or(0);
    Ok(changed > 0)
}
