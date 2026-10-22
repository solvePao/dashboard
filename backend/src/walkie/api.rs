use super::{repository::WalkieRepository, *};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub async fn handle(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    match dispatch(req, ctx).await {
        Ok(mut r) if r.status_code() != 101 => {
            // Responses fetched from a Durable Object have immutable headers.
            // Rebuild ordinary responses before the router attaches CORS headers.
            let status = r.status_code();
            let content_type = r
                .headers()
                .get("Content-Type")?
                .unwrap_or_else(|| "application/json".into());
            let fresh = Response::from_bytes(r.bytes().await?)?.with_status(status);
            fresh.headers().set("Content-Type", &content_type)?;
            Ok(fresh)
        }
        Ok(r) => Ok(r),
        Err(e) => e.to_response(),
    }
}

async fn dispatch(mut req: Request, ctx: RouteContext<()>) -> AppResult<Response> {
    let db = ctx.env.d1("dashboard").map_err(db_error)?;
    let repo = WalkieRepository(&db);
    let path = req.path();
    let method = req.method();
    let mut cleanup_pending = false;
    let group = ctx.param("group").cloned().unwrap_or_default();
    // Walkie signalling no longer has a public socket: it rides the single
    // gateway socket (`/api/live/device/:token`, channel `walkie`).
    let token = crate::utils::parse_bearer_header(
        req.headers()
            .get("Authorization")
            .map_err(db_error)?
            .as_deref(),
    )
    .ok_or_else(|| AppError::unauthorized("Authorization required."))?;
    if path.ends_with("/diagnostics") {
        let uid = crate::db::get_device_token_owner(&db, &token)
            .await.map_err(db_error)?
            .ok_or_else(|| AppError::unauthorized("Device authorization expired."))?;
        repo.rate_limit(&format!("diagnostics:{uid}"), 60).await?;
        let body: Value = req.json().await
            .map_err(|_| AppError::bad_request("Invalid JSON body."))?;
        let event = body.get("event").and_then(Value::as_str).unwrap_or("");
        let status = body.get("status").and_then(Value::as_str).unwrap_or("");
        if !["enable", "pause", "hidden", "pagehide", "unmount", "group_changed", "selection_lost"].contains(&event)
            || !["activation", "connecting", "reconnecting", "ready", "calling", "ringing", "talking", "listening", "busy", "error"].contains(&status) {
            return Err(AppError::bad_request("Invalid diagnostic event."));
        }
        console_log!("walkie client lifecycle event={} status={}", event, status);
        return Response::from_json(&json!({"ok":true})).map_err(db_error);
    }
    if path.ends_with("/device-code") {
        let uid = crate::db::get_device_token_owner(&db, &token)
            .await
            .map_err(db_error)?
            .ok_or_else(|| AppError::unauthorized("Device authorization expired."))?;
        if method != Method::Get {
            let public_id = repo.public_id(&token).await?;
            repo.rate_limit(&format!("device-code:device:{public_id}"), 10)
                .await?;
            repo.rate_limit(&format!("device-code:user:{uid}"), 20)
                .await?;
            let ip = req
                .headers()
                .get("CF-Connecting-IP")
                .map_err(db_error)?
                .unwrap_or_else(|| "local".into());
            repo.rate_limit(
                &format!("device-code:ip:{:x}", Sha256::digest(ip.as_bytes())),
                30,
            )
            .await?;
        }
        return Response::from_json(&super::admission::own_code(&db, &token, method).await?)
            .map_err(db_error);
    }
    if path.ends_with("/wake") {
        let (assigned, _uid) = repo.device_group(&token).await?;
        let body: Value = req
            .json()
            .await
            .unwrap_or(serde_json::json!({}));
        let group_id = body
            .get("group_id")
            .and_then(Value::as_str)
            .unwrap_or(&assigned.id);
        repo.device_member(group_id, &token).await?;
        let alerted = super::repository::wake_group_devices(&ctx.env, &db, group_id, Some(&token)).await?;
        return Response::from_json(&json!({"ok": true, "alerted": alerted})).map_err(db_error);
    }
    if path.ends_with("/connect") || path.ends_with("/media") || path.ends_with("/availability") {
        let (assigned, uid) = repo.device_group(&token).await?;
        if path.ends_with("/availability") {
            if ctx.env.var("WALKIE_SFU_APP_ID").is_err()
                || ctx.env.secret("WALKIE_SFU_APP_SECRET").is_err()
            {
                return Err(AppError::service_unavailable(
                    "Walkie-talkie audio is not configured. Set the backend SFU app ID and secret.",
                ));
            }
            return Response::from_json(&json!({"group_name":assigned.name})).map_err(db_error);
        }
        let body: Value = req
            .json()
            .await
            .map_err(|_| AppError::bad_request("Invalid JSON body."))?;
        return super::room_call(
            &ctx.env,
            &assigned.id,
            if path.ends_with("/connect") {
                "/ticket"
            } else {
                "/media"
            },
            &json!({"token":token,"user_id":uid,"group_id":assigned.id,"body":body}),
        )
        .await;
    }
    if path.ends_with("/activity") || path.ends_with("/memberships") {
        let uid = crate::db::get_device_token_owner(&db, &token)
            .await
            .map_err(db_error)?
            .ok_or_else(|| AppError::unauthorized("Device authorization expired."))?;
        if path.ends_with("/memberships") {
            // Only this phone's memberships, never all groups owned by its account.
            let mut groups = repo.groups(uid, &token).await?;
            groups.retain(|g| g.joined);
            let device_id = repo.public_id(&token).await?;
            return Response::from_json(&json!({"groups":groups,"device_id":device_id}))
                .map_err(db_error);
        }
        let body: Value = req
            .json()
            .await
            .map_err(|_| AppError::bad_request("Invalid JSON body."))?;
        let id = body
            .get("group_id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::bad_request("Group required."))?;
        repo.device_member(id, &token).await?;
        return room_call(
            &ctx.env,
            id,
            "/ticket",
            &json!({"token":token,"group_id":id,"observe":true}),
        )
        .await;
    }
    if path.ends_with("/active") {
        let body: Value = req
            .json()
            .await
            .map_err(|_| AppError::bad_request("Invalid JSON body."))?;
        let secret = crate::utils::jwt_secret(&ctx.env)?;
        let dashboard = crate::utils::verify_jwt(&token, &secret, now() / 1000).is_ok();
        let (uid, device) = if dashboard {
            let user = crate::services::AuthService::authenticate(&token, &ctx.env, &db).await?;
            (
                user.id,
                body.get("device_token")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            )
        } else {
            let uid = crate::db::get_device_token_owner(&db, &token)
                .await
                .map_err(db_error)?
                .ok_or_else(|| AppError::unauthorized("Device authorization expired."))?;
            (uid, token.clone())
        };
        repo.rate_limit(
            &format!("active:{}", repo.owned_device(&device, uid).await?),
            30,
        )
        .await?;
        let id = body
            .get("group_id")
            .filter(|v| !v.is_null())
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| AppError::bad_request("Invalid group."))
            })
            .transpose()?;
        let pending = super::enrollment::activate(&db, &ctx.env, uid, &device, id).await?;
        return mutation_response(json!({"ok":true}), pending);
    }
    // Dashboard management deliberately excludes long-lived device credentials.
    let secret = crate::utils::jwt_secret(&ctx.env)?;
    crate::utils::verify_jwt(&token, &secret, now() / 1000)
        .map_err(|_| AppError::unauthorized("Sign in to manage groups."))?;
    let user = crate::services::AuthService::authenticate(&token, &ctx.env, &db).await?;
    let uid = user.id;
    if method == Method::Get {
        let selected = req
            .url()
            .map_err(db_error)?
            .query_pairs()
            .find(|(k, _)| k == "device_token")
            .map(|(_, v)| v.into_owned())
            .unwrap_or_default();
        let device_id = repo.owned_device(&selected, uid).await?;
        let mut groups = repo.groups(uid, &selected).await?;
        for g in &mut groups {
            if let Ok(mut response) = room_call(&ctx.env, &g.id, "/presence", &json!({})).await {
                if let Ok(clients) = response.json::<Vec<Value>>().await {
                    for member in &mut g.members {
                        member.online = clients
                            .iter()
                            .any(|c| c["device_id"].as_str() == Some(&member.device_id));
                        member.audio_active = clients.iter().any(|c| {
                            c["device_id"].as_str() == Some(&member.device_id)
                                && c["published"] == true
                        });
                    }
                }
            }
        }
        return Response::from_json(&json!({"groups":groups,"user_id":uid,"device_id":device_id}))
            .map_err(db_error);
    }
    let ip = req
        .headers()
        .get("CF-Connecting-IP")
        .map_err(db_error)?
        .unwrap_or_else(|| "local".into());
    let ip_hash = format!("{:x}", Sha256::digest(ip.as_bytes()));
    let action = if path.ends_with("/device-preview") || path.ends_with("/devices") {
        "admission"
    } else {
        "manage"
    };
    repo.rate_limit(
        &format!("{action}:user:{uid}"),
        if action == "manage" { 30 } else { 20 },
    )
    .await?;
    repo.rate_limit(
        &format!("{action}:ip:{ip_hash}"),
        if action == "manage" { 100 } else { 60 },
    )
    .await?;
    if path.ends_with("/groups") && method == Method::Post {
        let body: Value = req
            .json()
            .await
            .map_err(|_| AppError::bad_request("Invalid JSON body."))?;
        let group_name = crate::services::validation::normalize_group_name(
            body.get("name").and_then(Value::as_str),
        )?;
        let id = super::enrollment::create(&db, &ctx.env, uid, &group_name).await?;
        return mutation_response(json!({"group_id":id}), false);
    }
    if path.ends_with("/join") || path.ends_with("/invite") {
        return Err(AppError::new("Group invite codes are no longer supported. Generate a device code on the phone and ask the group owner to add it.",410));
    }
    if path.ends_with("/device-preview") || path.ends_with("/devices") {
        repo.owner(&group, uid).await?;
        let body: Value = req
            .json()
            .await
            .map_err(|_| AppError::bad_request("Invalid JSON body."))?;
        let code = body.get("code").and_then(Value::as_str).unwrap_or("");
        if path.ends_with("/device-preview") {
            return Response::from_json(&super::admission::preview(&db, uid, &group, code).await?)
                .map_err(db_error);
        }
        let device_id = body
            .get("device_id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::bad_request("Preview the device before adding it."))?;
        let (device, pending) =
            super::admission::admit(&db, &ctx.env, uid, &group, code, device_id).await?;
        return mutation_response(json!({"group_id":group,"device_id":device}), pending);
    } else if let Some(member) = ctx.param("member") {
        #[derive(Deserialize)]
        struct Target {
            token: String,
            user_id: i64,
        }
        let g = repo.group(&group).await?;
        let target=repo.rows::<Target>("SELECT m.token,t.user_id FROM walkie_device_members m JOIN walkie_devices d ON d.token=m.token JOIN device_tokens t ON t.token=m.token WHERE m.group_id=? AND d.device_id=?", &[D1Type::Text(&group),D1Type::Text(member)]).await?.into_iter().next();
        if let Some(target) = target {
            if uid != g.owner_id && uid != target.user_id {
                return Err(AppError::forbidden(
                    "Only the owner can remove another account's device.",
                ));
            }
            repo.execute(
                "DELETE FROM walkie_device_members WHERE group_id=? AND token=?",
                &[D1Type::Text(&group), D1Type::Text(&target.token)],
            )
            .await?;
            repo.execute(
                "DELETE FROM walkie_assignments WHERE group_id=? AND token=?",
                &[D1Type::Text(&group), D1Type::Text(&target.token)],
            )
            .await?;
            reconcile_device_after_group_removal(&repo, &target.token, Some(target.user_id)).await?;
            cleanup_pending = invalidate(&ctx.env, &group, json!({"token":target.token})).await?;
            super::repository::notify_device(&ctx.env, &target.token).await;
        } else if g.owner_id != uid {
            return Err(AppError::not_found("Device membership not found."));
        }
    } else {
        repo.owner(&group, uid).await?;
        if method == Method::Patch {
            let body: Value = req
                .json()
                .await
                .map_err(|_| AppError::bad_request("Invalid JSON body."))?;
            let group_name = crate::services::validation::normalize_group_name(
                body.get("name").and_then(Value::as_str),
            )?;
            repo.execute(
                "UPDATE walkie_groups SET name=? WHERE id=?",
                &[D1Type::Text(&group_name), D1Type::Text(&group)],
            )
            .await?;
            invalidate(&ctx.env, &group, json!({})).await?;
        } else if method == Method::Delete {
            let phones = assigned_phones(&repo, &group).await?;
            repo.execute(
                "DELETE FROM walkie_device_members WHERE group_id=?",
                &[D1Type::Text(&group)],
            )
            .await?;
            repo.execute(
                "DELETE FROM walkie_assignments WHERE group_id=?",
                &[D1Type::Text(&group)],
            )
            .await?;
            repo.execute(
                "DELETE FROM walkie_groups WHERE id=?",
                &[D1Type::Text(&group)],
            )
            .await?;
            for token in &phones {
                reconcile_device_after_group_removal(&repo, token, None).await?;
                super::repository::notify_device(&ctx.env, token).await;
            }
            let result = invalidate(&ctx.env, &group, json!({"all":true})).await;
            cleanup_pending = result?;
        }
    }
    Response::from_json(&json!({"ok":true,"cleanup_pending":cleanup_pending}))
        .map(|r| r.with_status(if cleanup_pending { 202 } else { 200 }))
        .map_err(db_error)
}

async fn reconcile_device_after_group_removal(
    repo: &WalkieRepository<'_>,
    token: &str,
    user_id: Option<i64>,
) -> AppResult<()> {
    #[derive(Deserialize)]
    struct RemainingGroup {
        group_id: String,
    }
    let remaining = repo
        .rows::<RemainingGroup>(
            "SELECT group_id FROM walkie_device_members WHERE token=? LIMIT 1",
            &[D1Type::Text(token)],
        )
        .await?;
    if let Some(next_group) = remaining.into_iter().next() {
        let config = json!({"group_id": next_group.group_id}).to_string();
        if let Some(uid) = user_id {
            repo.execute(
                "INSERT INTO walkie_assignments(token,group_id,user_id) VALUES(?,?,?) ON CONFLICT(token) DO UPDATE SET group_id=excluded.group_id",
                &[
                    D1Type::Text(token),
                    D1Type::Text(&next_group.group_id),
                    crate::db::helper::user_id_param(uid).map_err(db_error)?,
                ],
            )
            .await?;
        } else {
            repo.execute(
                "UPDATE walkie_assignments SET group_id=? WHERE token=?",
                &[D1Type::Text(&next_group.group_id), D1Type::Text(token)],
            )
            .await?;
        }
        repo.execute(
            "UPDATE device_widgets SET enabled=1, config_json=? WHERE token=? AND widget_key='walkie_talkie'",
            &[D1Type::Text(&config), D1Type::Text(token)],
        )
        .await?;
    } else {
        repo.execute(
            "DELETE FROM walkie_assignments WHERE token=?",
            &[D1Type::Text(token)],
        )
        .await?;
        repo.execute(
            "UPDATE device_widgets SET enabled=0, config_json='{}' WHERE token=? AND widget_key='walkie_talkie'",
            &[D1Type::Text(token)],
        )
        .await?;
    }
    Ok(())
}

async fn invalidate(env: &Env, group: &str, body: Value) -> AppResult<bool> {
    let res = super::room_call(env, group, "/invalidate", &body).await?;
    if res.status_code() >= 400 {
        return Err(AppError::service_unavailable(
            "Group updated; live audio cleanup is retrying. Refresh before retrying.",
        ));
    }
    Ok(res.status_code() == 202)
}

async fn assigned_phones(repo: &WalkieRepository<'_>, group: &str) -> AppResult<Vec<String>> {
    #[derive(Deserialize)]
    struct Phone {
        token: String,
    }
    Ok(repo
        .rows::<Phone>(
            "SELECT token FROM walkie_device_members WHERE group_id=?",
            &[D1Type::Text(group)],
        )
        .await?
        .into_iter()
        .map(|p| p.token)
        .collect())
}

fn mutation_response(mut value: Value, pending: bool) -> AppResult<Response> {
    value["cleanup_pending"] = json!(pending);
    Response::from_json(&value)
        .map(|r| r.with_status(if pending { 202 } else { 200 }))
        .map_err(db_error)
}
