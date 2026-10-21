//! Device-generated codes are consent to admission, never device credentials.
use super::{repository::WalkieRepository, *};
use crate::db::helper::user_id_param;
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
pub struct DeviceCode {
    pub code: String,
    pub expires_at: u64,
}
#[derive(Deserialize)]
struct CodeRecord {
    token: String,
    expires_at: u64,
    consumed_at: Option<u64>,
    redeemed_group: Option<String>,
}

fn valid_code(code: &str) -> AppResult<()> {
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::bad_request("Enter a six-digit device code."));
    }
    Ok(())
}

async fn authorized_phone(repo: &WalkieRepository<'_>, token: &str) -> AppResult<i64> {
    let uid = crate::db::get_device_token_owner(repo.0, token)
        .await
        .map_err(db_error)?
        .ok_or_else(|| AppError::unauthorized("Device authorization expired."))?;
    let enabled=repo.rows::<Value>("SELECT 1 FROM device_widgets WHERE token=? AND widget_key='walkie_talkie' AND enabled=1", &[D1Type::Text(token)]).await?;
    if enabled.is_empty() {
        return Err(AppError::forbidden(
            "Enable the walkie-talkie widget in your dashboard first.",
        ));
    }
    repo.public_id(token).await?;
    Ok(uid)
}

pub async fn own_code(db: &D1Database, token: &str, method: Method) -> AppResult<Value> {
    let repo = WalkieRepository(db);
    authorized_phone(&repo, token).await?;
    if method == Method::Delete {
        repo.execute(
            "DELETE FROM walkie_device_codes WHERE token=?",
            &[D1Type::Text(token)],
        )
        .await?;
        return Ok(json!({"device_code":null,"status":"none"}));
    }
    if method == Method::Get {
        #[derive(Deserialize)]
        struct Current {
            code: String,
            expires_at: u64,
            consumed_at: Option<u64>,
        }
        let row = repo
            .rows::<Current>(
                "SELECT code,expires_at,consumed_at FROM walkie_device_codes WHERE token=?",
                &[D1Type::Text(token)],
            )
            .await?
            .into_iter()
            .next();
        return Ok(match row {
            Some(c) if c.consumed_at.is_some() => json!({"device_code":null,"status":"used"}),
            Some(c) if c.expires_at > now() => {
                json!({"device_code":{"code":c.code,"expires_at":c.expires_at},"status":"active"})
            }
            Some(_) => json!({"device_code":null,"status":"expired"}),
            None => json!({"device_code":null,"status":"none"}),
        });
    }
    repo.execute(
        "DELETE FROM walkie_device_codes WHERE expires_at<=?",
        &[D1Type::Real(now() as f64)],
    )
    .await?;
    for _ in 0..20 {
        let mut bytes = [0u8; 4];
        getrandom::getrandom(&mut bytes)
            .map_err(|_| AppError::internal("Could not generate a device code."))?;
        let n = u32::from_le_bytes(bytes);
        if n >= u32::MAX - (u32::MAX % 1_000_000) {
            continue;
        }
        let code = format!("{:06}", n % 1_000_000);
        let expires = now() + 600_000;
        let result=db.prepare("INSERT INTO walkie_device_codes(token,code,expires_at) VALUES(?,?,?) ON CONFLICT(token) DO UPDATE SET code=excluded.code,expires_at=excluded.expires_at,consumed_at=NULL,redeemed_group=NULL")
            .bind_refs(&[D1Type::Text(token),D1Type::Text(&code),D1Type::Real(expires as f64)]).map_err(db_error)?.run().await;
        match result {
            Ok(_) => {
                return Ok(
                    json!({"device_code":DeviceCode{code,expires_at:expires},"status":"active"}),
                )
            }
            Err(e) if e.to_string().contains("UNIQUE") => continue,
            Err(e) => return Err(db_error(e)),
        }
    }
    Err(AppError::service_unavailable(
        "Could not allocate a device code. Retry.",
    ))
}

async fn target(
    repo: &WalkieRepository<'_>,
    uid: i64,
    group: &str,
    code: &str,
) -> AppResult<(CodeRecord, Member)> {
    repo.owner(group, uid).await?;
    valid_code(code)?;
    let record=repo.rows::<CodeRecord>("SELECT token,expires_at,consumed_at,redeemed_group FROM walkie_device_codes WHERE code=?", &[D1Type::Text(code)]).await?.into_iter().next()
        .ok_or_else(||AppError::not_found("Device code is invalid, expired, or cancelled."))?;
    if record.expires_at <= now() {
        return Err(AppError::not_found("Device code has expired."));
    }
    authorized_phone(repo, &record.token).await?;
    if record.consumed_at.is_some()
        && (record.redeemed_group.as_deref() != Some(group)
            || repo.member(group, &record.token).await.is_err())
    {
        return Err(AppError::not_found(
            "Device code has already been used. Request a new code from that phone.",
        ));
    }
    let profile = repo.profile(&record.token).await?;
    Ok((record, profile))
}

pub async fn preview(db: &D1Database, uid: i64, group: &str, code: &str) -> AppResult<Value> {
    let repo = WalkieRepository(db);
    let (_, device) = target(&repo, uid, group, code).await?;
    let g = repo.group(group).await?;
    Ok(
        json!({"group_id":group,"group_name":g.name,"member_count":repo.members(group).await?.len(),"device":device}),
    )
}

fn args<'a>(
    group: &'a str,
    uid: i64,
    code: &'a str,
    device: &'a str,
    time: f64,
) -> AppResult<Vec<D1Type<'a>>> {
    Ok(vec![
        D1Type::Text(group),
        user_id_param(uid).map_err(db_error)?,
        D1Type::Text(code),
        D1Type::Text(device),
        D1Type::Real(time),
    ])
}

pub async fn admit(
    db: &D1Database,
    env: &Env,
    uid: i64,
    group: &str,
    code: &str,
    device_id: &str,
) -> AppResult<(String, bool)> {
    let repo = WalkieRepository(db);
    let (record, profile) = target(&repo, uid, group, code).await?;
    if profile.device_id != device_id {
        return Err(AppError::new(
            "The device changed. Preview its code again.",
            409,
        ));
    }
    let old = repo.active_id(&record.token).await?;
    let time = now() as f64;
    // Recheck every authority inside the transaction, including the enabled widget and
    // unexpired device credential. Consumed receipts are valid only for surviving membership.
    let eligible="SELECT c.token,c.consumed_at,t.user_id FROM walkie_device_codes c JOIN walkie_devices d ON d.token=c.token JOIN device_tokens t ON t.token=c.token JOIN device_widgets w ON w.token=c.token AND w.widget_key='walkie_talkie' AND w.enabled=1 JOIN walkie_groups g ON g.id=? AND g.owner_id=? WHERE c.code=? AND d.device_id=? AND c.expires_at>? AND t.expires_at>strftime('%Y-%m-%dT%H:%M:%fZ','now') AND (c.consumed_at IS NULL OR (c.redeemed_group=g.id AND EXISTS(SELECT 1 FROM walkie_device_members m WHERE m.group_id=g.id AND m.token=c.token)))";
    let insert=format!("INSERT INTO walkie_device_members(group_id,token) SELECT ?,e.token FROM ({eligible}) e WHERE e.consumed_at IS NULL ON CONFLICT(group_id,token) DO NOTHING");
    let mut insert_args = vec![D1Type::Text(group)];
    insert_args.extend(args(group, uid, code, device_id, time)?);
    let activate=format!("INSERT INTO walkie_assignments(token,group_id,user_id) SELECT e.token,?,e.user_id FROM ({eligible}) e WHERE e.consumed_at IS NULL AND NOT EXISTS(SELECT 1 FROM walkie_assignments a WHERE a.token=e.token) AND (SELECT count(*) FROM walkie_device_members m WHERE m.token=e.token)=1");
    let mut active_args = vec![D1Type::Text(group)];
    active_args.extend(args(group, uid, code, device_id, time)?);
    let widget=format!("UPDATE device_widgets SET config_json=json_object('group_id',?) WHERE widget_key='walkie_talkie' AND token IN(SELECT e.token FROM ({eligible}) e WHERE e.consumed_at IS NULL AND EXISTS(SELECT 1 FROM walkie_assignments a WHERE a.token=e.token AND a.group_id=?))");
    let mut widget_args = vec![D1Type::Text(group)];
    widget_args.extend(args(group, uid, code, device_id, time)?);
    widget_args.push(D1Type::Text(group));
    let consume=format!("UPDATE walkie_device_codes SET consumed_at=COALESCE(consumed_at,?),redeemed_group=? WHERE token IN(SELECT e.token FROM ({eligible}) e WHERE EXISTS(SELECT 1 FROM walkie_device_members m WHERE m.token=e.token AND m.group_id=?)) RETURNING token");
    let mut consume_args = vec![D1Type::Real(time), D1Type::Text(group)];
    consume_args.extend(args(group, uid, code, device_id, time)?);
    consume_args.push(D1Type::Text(group));
    let results = db
        .batch(vec![
            db.prepare(insert)
                .bind_refs(&insert_args)
                .map_err(db_error)?,
            db.prepare(activate)
                .bind_refs(&active_args)
                .map_err(db_error)?,
            db.prepare(widget)
                .bind_refs(&widget_args)
                .map_err(db_error)?,
            db.prepare(consume)
                .bind_refs(&consume_args)
                .map_err(db_error)?,
        ])
        .await
        .map_err(|e| {
            if e.to_string().contains("walkie_group_full") {
                AppError::new("This group already has five devices.", 409)
            } else {
                db_error(e)
            }
        })?;
    if results[3].results::<Value>().map_err(db_error)?.is_empty() {
        return Err(AppError::not_found(
            "Device code is no longer available. Preview a new code.",
        ));
    }
    let mut pending = super::enrollment::changed(db, env, &record.token, old).await?;
    match room_call(env, group, "/invalidate", &json!({})).await {
        Ok(r) if r.status_code() == 200 => {}
        _ => {
            console_error!("walkie admission metadata cleanup pending");
            pending = true;
        }
    }
    Ok((profile.device_id, pending))
}
