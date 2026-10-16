use worker::*;

use super::queries;
use crate::models::auth::User;
use super::helper::{text_or_null, user_id_param};

pub async fn get_user_by_sub(db: &D1Database, google_sub: &str) -> Result<Option<User>> {
    let stmt = db.prepare(queries::get_user_by_sub_sql());
    let args = [D1Type::Text(google_sub)];
    let stmt = stmt.bind_refs(&args)?;
    stmt.first::<User>(None).await
}

pub async fn get_user_by_id(db: &D1Database, id: i64) -> Result<Option<User>> {
    let stmt = db.prepare(queries::get_user_by_id_sql());
    let args = [user_id_param(id)?];
    let stmt = stmt.bind_refs(&args)?;
    stmt.first::<User>(None).await
}

pub async fn upsert_user(
    db: &D1Database,
    google_sub: &str,
    email: &str,
    name: Option<&str>,
    picture_url: Option<&str>,
) -> Result<User> {
    let args = [
        D1Type::Text(google_sub),
        D1Type::Text(email),
        text_or_null(name),
        text_or_null(picture_url),
    ];
    let stmt = db.prepare(queries::upsert_user_sql());
    stmt.bind_refs(&args)?.run().await?;

    get_user_by_sub(db, google_sub)
        .await?
        .ok_or_else(|| Error::RustError("Failed to load user after upsert.".to_string()))
}

pub async fn get_user_by_device_token(db: &D1Database, token: &str) -> Result<Option<User>> {
    let stmt = db.prepare(queries::get_user_by_device_token_sql());
    let args = [D1Type::Text(token)];
    let user = stmt.bind_refs(&args)?.first::<User>(None).await?;

    if user.is_some() {
        // Update last_used_at
        let update_stmt = db.prepare(queries::touch_device_token_sql());
        let update_args = [D1Type::Text(token)];
        let _ = update_stmt.bind_refs(&update_args)?.run().await;
    }

    Ok(user)
}

