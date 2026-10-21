//! Group creation and active-device configuration.
use super::{
    repository::{invalidate_device, notify_device, WalkieRepository},
    *,
};
use crate::db::helper::user_id_param;
use serde_json::json;

pub async fn changed(
    db: &D1Database,
    env: &Env,
    token: &str,
    old: Option<String>,
) -> AppResult<bool> {
    let current = WalkieRepository(db).active_id(token).await?;
    let mut pending = false;
    if old != current {
        if let Some(id) = old {
            pending |= invalidate_device(env, &id, token).await;
        }
        // Close an activity-only connection before this device establishes its media connection.
        if let Some(id) = current {
            pending |= invalidate_device(env, &id, token).await;
        }
    }
    notify_device(env, token).await;
    Ok(pending)
}

pub async fn activate(
    db: &D1Database,
    env: &Env,
    uid: i64,
    token: &str,
    group: Option<&str>,
) -> AppResult<bool> {
    let repo = WalkieRepository(db);
    repo.owned_device(token, uid).await?;
    let old = repo.active_id(token).await?;
    if let Some(id) = group {
        repo.member(id, token).await?;
    }
    let mut statements = vec![db
        .prepare("DELETE FROM walkie_assignments WHERE token=?")
        .bind_refs(&[D1Type::Text(token)])
        .map_err(db_error)?];
    if let Some(id) = group {
        statements.push(
            db.prepare("INSERT INTO walkie_assignments(token,group_id,user_id) VALUES(?,?,?)")
                .bind_refs(&[
                    D1Type::Text(token),
                    D1Type::Text(id),
                    user_id_param(uid).map_err(db_error)?,
                ])
                .map_err(db_error)?,
        );
    }
    let config = json!({"group_id":group}).to_string();
    statements.push(db.prepare("INSERT INTO device_widgets(token,widget_key,enabled,config_json) VALUES(?,'walkie_talkie',1,?) ON CONFLICT(token,widget_key) DO UPDATE SET enabled=1,config_json=excluded.config_json")
        .bind_refs(&[D1Type::Text(token),D1Type::Text(&config)]).map_err(db_error)?);
    db.batch(statements).await.map_err(db_error)?;
    changed(db, env, token, old).await
}

pub async fn create(db: &D1Database, env: &Env, uid: i64, name: &str) -> AppResult<String> {
    let id = random_id()?;
    WalkieRepository(db)
        .execute(
            "INSERT INTO walkie_groups(id,name,owner_id,created_at) VALUES(?,?,?,?)",
            &[
                D1Type::Text(&id),
                D1Type::Text(name),
                user_id_param(uid).map_err(db_error)?,
                D1Type::Real(now() as f64),
            ],
        )
        .await?;
    let _ = super::repository::auto_enroll_account_devices(db, env, uid, &id).await;
    Ok(id)
}
