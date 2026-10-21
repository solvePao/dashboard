use super::*;
use crate::db::helper::user_id_param;
use crate::models::device::DeviceWidget;
use crate::services::store::{DeviceNotifier, HubNotifier};
use crate::services::{DoDeviceNotifier, DoHubNotifier};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

pub struct WalkieRepository<'a>(pub &'a D1Database);

impl WalkieRepository<'_> {
    pub async fn rows<T: DeserializeOwned>(
        &self,
        sql: &str,
        args: &[D1Type<'_>],
    ) -> AppResult<Vec<T>> {
        self.0
            .prepare(sql)
            .bind_refs(args)
            .map_err(db_error)?
            .all()
            .await
            .map_err(db_error)?
            .results()
            .map_err(db_error)
    }

    pub async fn execute(&self, sql: &str, args: &[D1Type<'_>]) -> AppResult<()> {
        self.0
            .prepare(sql)
            .bind_refs(args)
            .map_err(db_error)?
            .run()
            .await
            .map_err(db_error)?;
        Ok(())
    }

    pub async fn group(&self, group: &str) -> AppResult<Group> {
        self.rows::<Group>(
            "SELECT * FROM walkie_groups WHERE id=?",
            &[D1Type::Text(group)],
        )
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("Group is unavailable."))
    }

    pub async fn owner(&self, group: &str, uid: i64) -> AppResult<Group> {
        let g = self.group(group).await?;
        if g.owner_id != uid {
            return Err(AppError::forbidden("Only the group owner can do this."));
        }
        Ok(g)
    }

    pub async fn owned_device(&self, token: &str, uid: i64) -> AppResult<String> {
        if crate::db::get_device_token_owner(self.0, token)
            .await
            .map_err(db_error)?
            != Some(uid)
        {
            return Err(AppError::not_found(
                "Device not found or authorization expired.",
            ));
        }
        self.public_id(token).await
    }

    pub async fn public_id(&self, token: &str) -> AppResult<String> {
        self.execute(
            "INSERT INTO walkie_devices(token) VALUES(?) ON CONFLICT(token) DO NOTHING",
            &[D1Type::Text(token)],
        )
        .await?;
        #[derive(Deserialize)]
        struct Identity {
            device_id: String,
        }
        self.rows::<Identity>(
            "SELECT device_id FROM walkie_devices WHERE token=?",
            &[D1Type::Text(token)],
        )
        .await?
        .into_iter()
        .next()
        .map(|d| d.device_id)
        .ok_or_else(|| AppError::not_found("Device not found."))
    }

    pub async fn member(&self, group: &str, token: &str) -> AppResult<Group> {
        self.rows::<Group>("SELECT g.* FROM walkie_groups g JOIN walkie_device_members m ON m.group_id=g.id WHERE g.id=? AND m.token=?",
            &[D1Type::Text(group), D1Type::Text(token)]).await?.into_iter().next()
            .ok_or_else(|| AppError::not_found("This device is not a member of the group."))
    }

    pub async fn profile(&self, token: &str) -> AppResult<Member> {
        self.profiles("WHERE t.token=?", &[D1Type::Text(token)])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::not_found("Device not found."))
    }

    async fn profiles(&self, clause: &str, args: &[D1Type<'_>]) -> AppResult<Vec<Member>> {
        let sql=format!("SELECT d.device_id,t.user_id,COALESCE(NULLIF(u.name,''),'Member') AS name,t.device_name,u.picture_url FROM walkie_devices d JOIN device_tokens t ON t.token=d.token JOIN users u ON u.id=t.user_id {clause}");
        let mut members = self.rows::<Member>(&sql, args).await?;
        for member in &mut members {
            member.first_name = member
                .name
                .split_whitespace()
                .next()
                .unwrap_or("Member")
                .to_string();
            member.name = member.first_name.clone();
        }
        Ok(members)
    }

    pub async fn members(&self, group: &str) -> AppResult<Vec<Member>> {
        self.profiles("JOIN walkie_device_members m ON m.token=t.token WHERE m.group_id=? ORDER BY t.created_at,d.device_id", &[D1Type::Text(group)]).await
    }

    pub async fn groups(&self, uid: i64, token: &str) -> AppResult<Vec<Group>> {
        let mut groups = self.rows::<Group>("SELECT g.*, EXISTS(SELECT 1 FROM walkie_device_members m WHERE m.group_id=g.id AND m.token=?) AS joined, EXISTS(SELECT 1 FROM walkie_assignments a WHERE a.group_id=g.id AND a.token=?) AS active FROM walkie_groups g WHERE g.owner_id=? OR EXISTS(SELECT 1 FROM walkie_device_members m WHERE m.group_id=g.id AND m.token=?) ORDER BY g.created_at",
            &[D1Type::Text(token),D1Type::Text(token),user_id_param(uid).map_err(db_error)?,D1Type::Text(token)]).await?;
        for g in &mut groups {
            g.members = self.members(&g.id).await?;
        }
        Ok(groups)
    }

    pub async fn device_member(&self, group: &str, token: &str) -> AppResult<(Group, i64)> {
        let uid = crate::db::get_device_token_owner(self.0, token)
            .await
            .map_err(db_error)?
            .ok_or_else(|| AppError::unauthorized("Device authorization expired."))?;
        let enabled = self.rows::<serde_json::Value>("SELECT token FROM device_widgets WHERE token=? AND widget_key='walkie_talkie' AND enabled=1", &[D1Type::Text(token)]).await?;
        if enabled.is_empty() {
            return Err(AppError::forbidden(
                "Walkie-talkie is disabled on this device.",
            ));
        }
        Ok((self.member(group, token).await?, uid))
    }

    pub async fn active_id(&self, token: &str) -> AppResult<Option<String>> {
        #[derive(Deserialize)]
        struct Active {
            group_id: String,
        }
        Ok(self
            .rows::<Active>(
                "SELECT group_id FROM walkie_assignments WHERE token=?",
                &[D1Type::Text(token)],
            )
            .await?
            .into_iter()
            .next()
            .map(|a| a.group_id))
    }

    pub async fn device_group(&self, token: &str) -> AppResult<(Group, i64)> {
        let id = self.active_id(token).await?.ok_or_else(|| {
            AppError::forbidden("Choose an active walkie-talkie group for this device.")
        })?;
        let result = self.device_member(&id, token).await?;
        let valid=self.rows::<serde_json::Value>("SELECT token FROM device_widgets WHERE token=? AND widget_key='walkie_talkie' AND json_extract(config_json,'$.group_id')=?", &[D1Type::Text(token),D1Type::Text(&id)]).await?;
        if valid.is_empty() {
            return Err(AppError::forbidden(
                "The active group changed. Refresh device settings.",
            ));
        }
        Ok(result)
    }

    pub async fn rate_limit(&self, bucket: &str, limit: i32) -> AppResult<()> {
        #[derive(Deserialize)]
        struct Count {
            attempts: i32,
        }
        let window = (now() / 60_000) as i32;
        let count = self.0.prepare("INSERT INTO walkie_rate_limits(bucket,window,attempts) VALUES(?,?,1) ON CONFLICT(bucket) DO UPDATE SET attempts=CASE WHEN window=excluded.window THEN attempts+1 ELSE 1 END, window=excluded.window RETURNING attempts")
            .bind_refs(&[D1Type::Text(bucket), D1Type::Integer(window)]).map_err(db_error)?.first::<Count>(None).await.map_err(db_error)?;
        if count.is_some_and(|c| c.attempts > limit) {
            return Err(AppError::too_many_requests(
                "Too many attempts. Wait a minute and retry.",
            ));
        }
        // Bound retained rate-limit state without keeping historical activity.
        self.execute(
            "DELETE FROM walkie_rate_limits WHERE window < ?",
            &[D1Type::Integer(window - 2)],
        )
        .await
    }
}

/// The full widget set and its authorization assignment commit in one D1 batch.
pub async fn save_widgets(
    db: &D1Database,
    env: &Env,
    uid: i64,
    token: &str,
    widgets: Vec<DeviceWidget>,
) -> AppResult<(Vec<DeviceWidget>, bool)> {
    if crate::db::get_device_token_owner(db, token)
        .await
        .map_err(db_error)?
        != Some(uid)
    {
        return Err(AppError::not_found("Device not found."));
    }
    let repo = WalkieRepository(db);
    let selected = widgets.iter().find(|w| w.widget_key == "walkie_talkie");
    let group = selected
        .and_then(|w| w.config.get("group_id"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if let Some(w) = selected {
        if w.config
            .get("group_id")
            .is_some_and(|v| !v.is_null() && !v.is_string())
        {
            return Err(AppError::bad_request("Invalid walkie-talkie group."));
        }
    }
    if let Some(g) = group {
        repo.member(g, token).await?;
    }
    #[derive(Deserialize)]
    struct Assignment {
        token: String,
        group_id: String,
    }
    let mut affected: Vec<Assignment> = repo
        .rows(
            "SELECT token,group_id FROM walkie_assignments WHERE token=?",
            &[D1Type::Text(token)],
        )
        .await?;
    let assignment_changed = affected.first().map(|a| a.group_id.as_str()) != group;
    let mut statements = Vec::new();
    if assignment_changed {
        statements.push(
            db.prepare("DELETE FROM walkie_assignments WHERE token=?")
                .bind_refs(&[D1Type::Text(token)])
                .map_err(db_error)?,
        );
    }
    statements.push(
        db.prepare("DELETE FROM device_widgets WHERE token=?")
            .bind_refs(&[D1Type::Text(token)])
            .map_err(db_error)?,
    );
    for w in &widgets {
        let config = if w.widget_key == "walkie_talkie" {
            json!({"group_id":w.config.get("group_id").and_then(Value::as_str)})
        } else {
            w.config.clone()
        };
        statements.push(
            db.prepare(
                "INSERT INTO device_widgets(token,widget_key,enabled,config_json) VALUES(?,?,?,?)",
            )
            .bind_refs(&[
                D1Type::Text(token),
                D1Type::Text(&w.widget_key),
                D1Type::Integer(i32::from(w.enabled)),
                D1Type::Text(&config.to_string()),
            ])
            .map_err(db_error)?,
        );
    }
    if let Some(g) = group.filter(|_| assignment_changed) {
        statements.push(
            db.prepare("INSERT INTO walkie_assignments(token,group_id,user_id) VALUES(?,?,?)")
                .bind_refs(&[
                    D1Type::Text(token),
                    D1Type::Text(g),
                    user_id_param(uid).map_err(db_error)?,
                ])
                .map_err(db_error)?,
        );
        affected.push(Assignment {
            token: token.into(),
            group_id: g.into(),
        });
    }
    db.batch(statements).await.map_err(db_error)?;
    if !assignment_changed {
        affected.clear();
    }
    if selected.is_none_or(|w| !w.enabled) {
        #[derive(Deserialize)]
        struct Membership {
            group_id: String,
        }
        for m in repo
            .rows::<Membership>(
                "SELECT group_id FROM walkie_device_members WHERE token=?",
                &[D1Type::Text(token)],
            )
            .await?
        {
            if !affected.iter().any(|a| a.group_id == m.group_id) {
                affected.push(Assignment {
                    token: token.into(),
                    group_id: m.group_id,
                });
            }
        }
    }
    let mut cleanup_pending = false;
    for a in &affected {
        // An old session must never survive a disable/reassign, even to the same group.
        cleanup_pending |= invalidate_device(env, &a.group_id, &a.token).await;
        notify_device(env, &a.token).await;
    }
    notify_device(env, token).await;
    if let Ok(hub) = DoHubNotifier::from_env(env) {
        hub.notify_hub(
            uid,
            crate::common::DeviceLifecycleKind::WidgetsUpdated,
            Some(token),
        )
        .await;
    }
    Ok((
        crate::db::get_device_widgets(db, token)
            .await
            .map_err(db_error)?,
        cleanup_pending,
    ))
}

pub async fn notify_device(env: &Env, token: &str) {
    if let Ok(notifier) = DoDeviceNotifier::from_env(env) {
        notifier.notify_widgets_updated(token).await;
    }
}

pub async fn wake_group_devices(
    env: &Env,
    db: &D1Database,
    group_id: &str,
    sender_token: Option<&str>,
) -> AppResult<usize> {
    let repo = WalkieRepository(db);
    #[derive(Deserialize)]
    struct MemberToken {
        token: String,
    }
    let members = repo
        .rows::<MemberToken>(
            "SELECT token FROM walkie_device_members WHERE group_id=?",
            &[D1Type::Text(group_id)],
        )
        .await?;
    let mut count = 0;
    if let Ok(notifier) = DoDeviceNotifier::from_env(env) {
        for m in members {
            if sender_token.is_none_or(|st| st != m.token) {
                notifier.notify_walkie_activate(&m.token, group_id).await;
                count += 1;
            }
        }
    }
    Ok(count)
}

pub async fn auto_enroll_account_devices(
    db: &D1Database,
    env: &Env,
    uid: i64,
    group_id: &str,
) -> AppResult<()> {
    let repo = WalkieRepository(db);
    #[derive(Deserialize)]
    struct Count {
        count: i64,
    }
    let current_count = repo
        .rows::<Count>(
            "SELECT count(*) as count FROM walkie_device_members WHERE group_id=?",
            &[D1Type::Text(group_id)],
        )
        .await?
        .first()
        .map(|c| c.count)
        .unwrap_or(0);

    let remaining = 5 - current_count;
    if remaining <= 0 {
        return Ok(());
    }

    #[derive(Deserialize)]
    struct TokenRow {
        token: String,
    }
    let unassigned = repo
        .rows::<TokenRow>(
            "SELECT t.token FROM device_tokens t \
             WHERE t.user_id=? AND t.expires_at>strftime('%Y-%m-%dT%H:%M:%fZ','now') \
             AND NOT EXISTS(SELECT 1 FROM walkie_device_members m WHERE m.group_id=? AND m.token=t.token) \
             LIMIT ?",
            &[user_id_param(uid).map_err(db_error)?, D1Type::Text(group_id), D1Type::Integer(remaining as i32)],
        )
        .await?;

    if unassigned.is_empty() {
        return Ok(());
    }

    for row in &unassigned {
        repo.execute(
            "INSERT INTO walkie_devices(token) VALUES(?) ON CONFLICT(token) DO NOTHING",
            &[D1Type::Text(&row.token)],
        )
        .await?;

        repo.execute(
            "INSERT INTO walkie_device_members(group_id,token) VALUES(?,?) ON CONFLICT(group_id,token) DO NOTHING",
            &[D1Type::Text(group_id), D1Type::Text(&row.token)],
        )
        .await?;

        let has_assignment = !repo
            .rows::<TokenRow>(
                "SELECT token FROM walkie_assignments WHERE token=?",
                &[D1Type::Text(&row.token)],
            )
            .await?
            .is_empty();

        if !has_assignment {
            repo.execute(
                "INSERT INTO walkie_assignments(token,group_id,user_id) VALUES(?,?,?)",
                &[D1Type::Text(&row.token), D1Type::Text(group_id), user_id_param(uid).map_err(db_error)?],
            )
            .await?;
        }

        let active_group = repo.active_id(&row.token).await?;
        let config = json!({"group_id": active_group}).to_string();
        repo.execute(
            "INSERT INTO device_widgets(token,widget_key,enabled,config_json) VALUES(?,'walkie_talkie',1,?) \
             ON CONFLICT(token,widget_key) DO UPDATE SET enabled=1,config_json=excluded.config_json",
            &[D1Type::Text(&row.token), D1Type::Text(&config)],
        )
        .await?;

        notify_device(env, &row.token).await;
    }

    Ok(())
}

/// Authorization has already committed. Surface delayed teardown separately from that success.
pub async fn invalidate_device(env: &Env, group: &str, token: &str) -> bool {
    match super::room_call(env, group, "/invalidate", &json!({"token":token})).await {
        Ok(response) if response.status_code() == 200 => false,
        _ => {
            console_error!("walkie device media cleanup pending");
            true
        }
    }
}
