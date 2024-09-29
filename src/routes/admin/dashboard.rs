use crate::authentication::UserId;
use actix_web::{
    http::header::ContentType,
    web::{self, ReqData},
    HttpResponse,
};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::utils::e500;

pub async fn admin_dashboard(
    user_id: ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = get_username(*user_id.into_inner(), &pool)
        .await
        .map_err(e500)?;
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
    <li><a href="/admin/newsletters">Send a newsletter</a></li>
    <li><a href="/admin/password">Change password</a></li>
    <form name="logoutForm" action="/admin/logout" method="post">
        <input type="submit" value="logout">
    </form>
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
