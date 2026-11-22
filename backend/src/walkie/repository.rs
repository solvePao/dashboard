use super::*;
use crate::db::helper::user_id_param;
use crate::models::device::DeviceWidget;
use crate::services::store::{DeviceNotifier, HubNotifier};
use crate::services::{DoDeviceNotifier, DoHubNotifier};
use futures_util::stream::{self, StreamExt};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::collections::HashMap;

const PROFILE_COLUMNS: &str = "d.device_id,t.user_id,COALESCE(NULLIF(u.name,''),'Member') AS name,t.device_name,u.picture_url";
const PROFILE_TABLES: &str =
    "walkie_devices d JOIN device_tokens t ON t.token=d.token JOIN users u ON u.id=t.user_id";

pub struct WalkieRepository<'a>(pub &'a D1Database);

impl WalkieRepository<'_> {
    pub async fn first_owned_group_id(&self, user_id: i64) -> AppResult<Option<String>> {
        #[derive(Deserialize)]
        struct OwnedGroup {
            id: String,
        }

        Ok(self
            .rows::<OwnedGroup>(
                "SELECT id FROM walkie_groups WHERE owner_id=? ORDER BY created_at ASC LIMIT 1",
                &[user_id_param(user_id).map_err(db_error)?],
            )
            .await?
            .into_iter()
            .next()
            .map(|group| group.id))
    }

    pub async fn device_group_ids(&self, user_id: i64, token: &str) -> AppResult<Vec<String>> {
        #[derive(Deserialize)]
        struct GroupMembership {
            group_id: String,
        }

        let user_param = user_id_param(user_id).map_err(|_| AppError::internal("Invalid user."))?;
        let memberships = self
            .rows::<GroupMembership>(
                "SELECT m.group_id FROM walkie_device_members m JOIN device_tokens t ON t.token=m.token WHERE m.token=? AND t.user_id=?",
                &[D1Type::Text(token), user_param],
            )
            .await?;
        Ok(memberships
            .into_iter()
            .map(|membership| membership.group_id)
            .collect())
    }

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
        #[derive(Deserialize)]
        struct Identity {
            device_id: String,
        }

        let args = [D1Type::Text(token)];
        let lookup = self
            .0
            .prepare("SELECT device_id FROM walkie_devices WHERE token=?")
            .bind_refs(&args)
            .map_err(db_error)?;
        if let Some(identity) = lookup.first::<Identity>(None).await.map_err(db_error)? {
            return Ok(identity.device_id);
        }
        let insert = self
            .0
            .prepare("INSERT INTO walkie_devices(token) VALUES(?) ON CONFLICT(token) DO NOTHING")
            .bind_refs(&args)
            .map_err(db_error)?;
        let results = self.0.batch(vec![insert, lookup]).await.map_err(db_error)?;
        results[1]
            .results::<Identity>()
            .map_err(db_error)?
            .into_iter()
            .next()
            .map(|identity| identity.device_id)
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
        let sql = format!("SELECT {PROFILE_COLUMNS} FROM {PROFILE_TABLES} {clause}");
        let mut members = self.rows::<Member>(&sql, args).await?;
        for member in &mut members {
            member.normalize_name();
        }
        Ok(members)
    }

    pub async fn members(&self, group: &str) -> AppResult<Vec<Member>> {
        self.profiles("JOIN walkie_device_members m ON m.token=t.token WHERE m.group_id=? ORDER BY t.created_at,d.device_id", &[D1Type::Text(group)]).await
    }

    pub async fn groups(&self, uid: i64, token: &str, joined_only: bool) -> AppResult<Vec<Group>> {
        #[derive(Deserialize)]
        struct RosterEntry {
            group_id: String,
            #[serde(flatten)]
            member: Member,
        }

        let args = [
            D1Type::Text(token),
            user_id_param(uid).map_err(db_error)?,
            D1Type::Integer(i32::from(joined_only)),
        ];
        let joined = "EXISTS(SELECT 1 FROM walkie_device_members own WHERE own.group_id=g.id AND own.token=?1)";
        let visible = "WITH visible AS (
            SELECT id FROM walkie_groups WHERE ?3=0 AND owner_id=?2
            UNION SELECT group_id FROM walkie_device_members WHERE token=?1
        )";
        let groups = self.0.prepare(&format!(
            "{visible}
             SELECT g.*, {joined} AS joined,
                    EXISTS(SELECT 1 FROM walkie_assignments a WHERE a.group_id=g.id AND a.token=?1) AS active
             FROM visible v JOIN walkie_groups g ON g.id=v.id ORDER BY g.created_at"
        )).bind_refs(&args).map_err(db_error)?;
        let roster = self
            .0
            .prepare(&format!(
                "{visible}
             SELECT m.group_id, {PROFILE_COLUMNS} FROM {PROFILE_TABLES}
             JOIN walkie_device_members m ON m.token=t.token
             JOIN visible v ON v.id=m.group_id ORDER BY t.created_at,d.device_id"
            ))
            .bind_refs(&args)
            .map_err(db_error)?;
        let results = self.0.batch(vec![groups, roster]).await.map_err(db_error)?;
        let mut groups = results[0].results::<Group>().map_err(db_error)?;
        let mut members = HashMap::<String, Vec<Member>>::new();
        for mut entry in results[1].results::<RosterEntry>().map_err(db_error)? {
            entry.member.normalize_name();
            members
                .entry(entry.group_id)
                .or_default()
                .push(entry.member);
        }
        for group in &mut groups {
            group.members = members.remove(&group.id).unwrap_or_default();
        }
        Ok(groups)
    }

    pub async fn device_member(&self, group: &str, token: &str) -> AppResult<(Group, i64)> {
        self.device_access(token, Some(group)).await
    }

    async fn device_access(&self, token: &str, group: Option<&str>) -> AppResult<(Group, i64)> {
        #[derive(Deserialize)]
        struct Access {
            assignment: Option<String>,
            user_id: Option<i64>,
            widget_token: Option<String>,
            id: Option<String>,
            name: Option<String>,
            owner_id: Option<i64>,
            config_matches: Option<i32>,
        }

        let access = self.0.prepare(&format!(
            "SELECT a.group_id AS assignment, t.user_id, w.token AS widget_token,
                    g.id, g.name, g.owner_id,
                    CASE WHEN ?2 IS NULL AND g.id IS NOT NULL THEN
                        json_extract(w.config_json, '$.group_id') = a.group_id END AS config_matches
             FROM (SELECT ?1 AS token) AS requested
             LEFT JOIN walkie_assignments a ON a.token=requested.token AND ?2 IS NULL
             LEFT JOIN device_tokens t ON t.token=requested.token AND t.expires_at>{}
             LEFT JOIN device_widgets w ON w.token=t.token AND w.widget_key='walkie_talkie' AND w.enabled=1
             LEFT JOIN walkie_device_members m ON m.token=w.token AND m.group_id=COALESCE(?2,a.group_id)
             LEFT JOIN walkie_groups g ON g.id=m.group_id",
            crate::common::SQL_NOW,
        ))
        .bind_refs(&[D1Type::Text(token), group.map_or(D1Type::Null, D1Type::Text)])
        .map_err(db_error)?
        .first::<Access>(None).await.map_err(db_error)?
        .ok_or_else(|| AppError::internal("Could not check Caller access. Please retry."))?;

        if group.is_none() && access.assignment.is_none() {
            return Err(AppError::forbidden(
                "Choose an active walkie-talkie group for this device.",
            ));
        }
        let uid = access
            .user_id
            .ok_or_else(|| AppError::unauthorized("Device authorization expired."))?;
        if access.widget_token.is_none() {
            return Err(AppError::forbidden(
                "Walkie-talkie is disabled on this device.",
            ));
        }
        let (Some(id), Some(name), Some(owner_id)) = (access.id, access.name, access.owner_id)
        else {
            return Err(AppError::not_found(
                "This device is not a member of the group.",
            ));
        };
        if group.is_none() && access.config_matches != Some(1) {
            return Err(AppError::forbidden(
                "The active group changed. Refresh device settings.",
            ));
        }
        Ok((
            Group {
                id,
                name,
                owner_id,
                members: Vec::new(),
                joined: false,
                active: false,
            },
            uid,
        ))
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
        self.device_access(token, None).await
    }

    pub async fn rate_limit(&self, bucket: &str, limit: i32) -> AppResult<()> {
        #[derive(Deserialize)]
        struct Count {
            attempts: i32,
        }
        let window = (now() / 60_000) as i32;
        let increment = self.0.prepare("INSERT INTO walkie_rate_limits(bucket,window,attempts) VALUES(?,?,1) ON CONFLICT(bucket) DO UPDATE SET attempts=CASE WHEN window=excluded.window THEN attempts+1 ELSE 1 END, window=excluded.window RETURNING attempts")
            .bind_refs(&[D1Type::Text(bucket), D1Type::Integer(window)]).map_err(db_error)?;
        let prune = self
            .0
            .prepare("DELETE FROM walkie_rate_limits WHERE window < ?")
            .bind_refs(&[D1Type::Integer(window - 2)])
            .map_err(db_error)?;
        let results = self
            .0
            .batch(vec![increment, prune])
            .await
            .map_err(db_error)?;
        let counts = results[0].results::<Count>().map_err(db_error)?;
        if counts.first().is_some_and(|count| count.attempts > limit) {
            return Err(AppError::too_many_requests(
                "Too many attempts. Wait a minute and retry.",
            ));
        }
        Ok(())
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
        group_id: String,
    }
    let mut affected: Vec<Assignment> = repo
        .rows(
            "SELECT group_id FROM walkie_assignments WHERE token=?",
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
                    group_id: m.group_id,
                });
            }
        }
    }
    // An old session must never survive a disable/reassign, even to the same group.
    let cleanup_pending =
        invalidate_device_groups(env, token, affected.into_iter().map(|a| a.group_id)).await;
    let notify_hub = async {
        if let Ok(hub) = DoHubNotifier::from_env(env) {
            hub.notify_hub(
                uid,
                crate::common::DeviceLifecycleKind::WidgetsUpdated,
                Some(token),
            )
            .await;
        }
    };
    let (widgets, _, _) = futures_util::join!(
        crate::db::get_device_widgets(db, token),
        notify_device(env, token),
        notify_hub,
    );
    Ok((widgets.map_err(db_error)?, cleanup_pending))
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
    let Ok(notifier) = DoDeviceNotifier::from_env(env) else {
        return Ok(0);
    };
    let notifier = &notifier;
    let targets: Vec<_> = members
        .into_iter()
        .filter(|member| sender_token != Some(member.token.as_str()))
        .collect();
    let count = targets.len();
    stream::iter(targets)
        .for_each_concurrent(5, |member| async move {
            notifier
                .notify_walkie_activate(&member.token, group_id)
                .await;
        })
        .await;
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
    struct TokenRow {
        token: String,
    }
    let owner = user_id_param(uid).map_err(db_error)?;
    let expiry = crate::common::SQL_NOW;
    let unassigned = repo
        .rows::<TokenRow>(
            &format!("SELECT t.token FROM device_tokens t
             WHERE t.user_id=?1 AND t.expires_at>{expiry}
             AND NOT EXISTS(SELECT 1 FROM walkie_device_members m WHERE m.group_id=?2 AND m.token=t.token)
             LIMIT MAX(0, 5-(SELECT count(*) FROM walkie_device_members WHERE group_id=?2))"),
            &[owner, D1Type::Text(group_id)],
        )
        .await?;

    if unassigned.is_empty() {
        return Ok(());
    }

    let member_sql = format!(
        "INSERT INTO walkie_device_members(group_id,token)
        SELECT g.id,t.token FROM walkie_groups g JOIN device_tokens t ON t.token=?1
        WHERE g.id=?2 AND g.owner_id=?3 AND t.user_id=g.owner_id AND t.expires_at>{expiry}
        AND (SELECT count(*) FROM walkie_device_members WHERE group_id=g.id)<5
        ON CONFLICT(group_id,token) DO NOTHING"
    );
    let widget_sql = format!(
        "INSERT INTO device_widgets(token,widget_key,enabled,config_json)
        SELECT t.token,'walkie_talkie',1,json_object('group_id',a.group_id)
        FROM device_tokens t JOIN walkie_device_members m ON m.token=t.token
        LEFT JOIN walkie_assignments a ON a.token=t.token
        WHERE t.token=?1 AND m.group_id=?2 AND t.expires_at>{expiry}
        ON CONFLICT(token,widget_key) DO UPDATE SET enabled=1,config_json=excluded.config_json
        RETURNING token"
    );
    // Recheck expiry and available seats in the write transaction, since a
    // concurrent enrollment or revocation may have changed the candidate set.
    let mut statements = Vec::with_capacity(unassigned.len() * 4);
    for row in &unassigned {
        let token = D1Type::Text(&row.token);
        statements.push(
            db.prepare(
                "INSERT INTO walkie_devices(token)
            SELECT token FROM device_tokens WHERE token=?1 ON CONFLICT(token) DO NOTHING",
            )
            .bind_refs(&[token])
            .map_err(db_error)?,
        );
        statements.push(
            db.prepare(&member_sql)
                .bind_refs(&[
                    D1Type::Text(&row.token),
                    D1Type::Text(group_id),
                    user_id_param(uid).map_err(db_error)?,
                ])
                .map_err(db_error)?,
        );
        let membership = [D1Type::Text(&row.token), D1Type::Text(group_id)];
        statements.push(
            db.prepare(
                "INSERT INTO walkie_assignments(token,group_id,user_id)
            SELECT t.token,m.group_id,t.user_id FROM device_tokens t
            JOIN walkie_device_members m ON m.token=t.token
            WHERE t.token=?1 AND m.group_id=?2 ON CONFLICT(token) DO NOTHING",
            )
            .bind_refs(&membership)
            .map_err(db_error)?,
        );
        statements.push(
            db.prepare(&widget_sql)
                .bind_refs(&membership)
                .map_err(db_error)?,
        );
    }
    let results = db.batch(statements).await.map_err(db_error)?;
    let mut updated = Vec::new();
    for result in results.iter().skip(3).step_by(4) {
        updated.extend(result.results::<TokenRow>().map_err(db_error)?);
    }
    stream::iter(updated)
        .for_each_concurrent(5, |row| async move {
            notify_device(env, &row.token).await;
        })
        .await;

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

pub async fn invalidate_device_groups(
    env: &Env,
    token: &str,
    groups: impl IntoIterator<Item = String>,
) -> bool {
    stream::iter(groups)
        .map(|group| async move { invalidate_device(env, &group, token).await })
        .buffer_unordered(5)
        .fold(false, |pending, result| async move { pending || result })
        .await
}
