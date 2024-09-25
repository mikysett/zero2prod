use actix_web::{http::header::ContentType, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn admin_dashboard(
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    let username = match session.get_username().map_err(e500)? {
        Some(username) => username,
        None => return Ok(see_other("/login")),
    };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!doctype html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8" />
        <title>Admin dashboard</title>
    </head>
    <body>
    <p>Welcome {username}!</p>
    <p>Available actions:</p>
    <ol>
    <li><a href="/admin/password">Change password</a></li>
    </ol>
    </body>
</html>"#,
        )))
}

pub async fn get_username(
    user_id: Uuid,
    pool: &PgPool,
) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username")?;

    Ok(row.username)
}
