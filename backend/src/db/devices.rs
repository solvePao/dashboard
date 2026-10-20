use worker::*;

use super::helper::user_id_param;
use super::queries;
use crate::models::device::{DeviceToken, PairingCode};

pub async fn upsert_pairing_code(
    db: &D1Database,
    session_id: &str,
    short_code: &str,
) -> Result<PairingCode> {
    let args = [D1Type::Text(session_id), D1Type::Text(short_code)];
    let stmt = db.prepare(queries::upsert_pairing_code_sql());
    stmt.bind_refs(&args)?.run().await?;

    let stmt = db.prepare(queries::get_pairing_code_sql());
    let args = [D1Type::Text(session_id)];
    stmt.bind_refs(&args)?
        .first::<PairingCode>(None)
        .await?
        .ok_or_else(|| Error::RustError("Failed to load pairing code after insert.".to_string()))
}

pub async fn get_pairing_code(
    db: &D1Database,
    session_id: &str,
) -> Result<Option<PairingCode>> {
    let stmt = db.prepare(queries::get_pairing_code_sql());
    let args = [D1Type::Text(session_id)];
    let stmt = stmt.bind_refs(&args)?;
    stmt.first::<PairingCode>(None).await
}

/// Resolves a typed 6-char code to its session. Full session ids are
/// handled by the caller and never reach this lookup.
pub async fn find_session_by_code(
    db: &D1Database,
    code: &str,
) -> Result<Option<PairingCode>> {
    let lower_code = queries::normalize_session_lookup_key(code);
    let stmt = db.prepare(queries::find_session_by_code_sql());
    let args = [D1Type::Text(&lower_code)];
    let stmt = stmt.bind_refs(&args)?;
    stmt.first::<PairingCode>(None).await
}

pub async fn insert_device_token(
    db: &D1Database,
    user_id: i64,
    device_token: &str,
    device_name: &str,
) -> Result<()> {
    let stmt = db.prepare(queries::insert_device_token_sql());
    stmt.bind_refs(&[
        D1Type::Text(device_token),
        user_id_param(user_id)?,
        D1Type::Text(device_name),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Compensation for a lost authorize race: the Durable Object refused
/// the commit, so the just-minted token must not survive orphaned.
pub async fn delete_device_token(db: &D1Database, device_token: &str) -> Result<()> {
    let stmt = db.prepare(queries::delete_device_token_sql());
    stmt.bind_refs(&[D1Type::Text(device_token)])?
        .run()
        .await?;
    Ok(())
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
    if changed > 0 {
        // Widget rows are cleaned explicitly (no FK-cascade reliance).
        let stmt = db.prepare(queries::delete_device_widgets_sql());
        stmt.bind_refs(&[D1Type::Text(token)])?.run().await?;
    }
    Ok(changed > 0)
}

/// Owner id of one device token, or `None` when unknown/expired.
pub async fn get_device_token_owner(db: &D1Database, token: &str) -> Result<Option<i64>> {
    // D1 binds user ids as integers; read back as i32 then widen.
    let stmt = db.prepare(queries::device_token_owner_sql());
    let row: Option<OwnerRow> = stmt
        .bind_refs(&[D1Type::Text(token)])?
        .first::<OwnerRow>(None)
        .await?;
    Ok(row.map(|r| r.user_id as i64))
}

#[derive(serde::Deserialize)]
struct OwnerRow {
    user_id: i32,
}

/// Widget rows for one device token, ordered by key.
pub async fn get_device_widgets(
    db: &D1Database,
    token: &str,
) -> Result<Vec<crate::models::device::DeviceWidget>> {
    let stmt = db.prepare(queries::get_device_widgets_sql());
    let result = stmt.bind_refs(&[D1Type::Text(token)])?.all().await?;
    let rows: Vec<WidgetRow> = result.results::<WidgetRow>()?;
    Ok(rows.into_iter().map(WidgetRow::into_widget).collect())
}

#[derive(serde::Deserialize)]
struct WidgetRow {
    widget_key: String,
    enabled: i64,
    config_json: String,
    #[allow(dead_code)]
    updated_at: String,
}

impl WidgetRow {
    fn into_widget(self) -> crate::models::device::DeviceWidget {
        crate::models::device::DeviceWidget {
            widget_key: self.widget_key,
            enabled: self.enabled != 0,
            config: serde_json::from_str(&self.config_json).unwrap_or(serde_json::json!({})),
        }
    }
}

/// Replaces the full widget set for one device token: delete-then-insert
/// so a PUT body is always the complete desired state (empty clears all).
pub async fn replace_device_widgets(
    db: &D1Database,
    token: &str,
    widgets: &[crate::models::device::DeviceWidget],
) -> Result<()> {
    let stmt = db.prepare(queries::delete_device_widgets_sql());
    stmt.bind_refs(&[D1Type::Text(token)])?.run().await?;
    for widget in widgets {
        let stmt = db.prepare(queries::upsert_device_widget_sql());
        let config = serde_json::to_string(&widget.config)
            .map_err(|e| Error::RustError(format!("Failed to encode widget config: {e}")))?;
        let enabled = if widget.enabled { 1 } else { 0 };
        stmt.bind_refs(&[
            D1Type::Text(token),
            D1Type::Text(&widget.widget_key),
            D1Type::Integer(enabled),
            D1Type::Text(&config),
        ])?
        .run()
        .await?;
    }
    Ok(())
}
