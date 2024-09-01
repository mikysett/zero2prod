use actix_web::{http::StatusCode, web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::routes::helpers::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Confirm a pending subsccriber",
    skip(pool, parameters)
)]
pub async fn confirm(
    pool: web::Data<PgPool>,
    parameters: web::Query<Parameters>,
) -> Result<HttpResponse, ConfirmError> {
    let id =
        get_subscriber_id_from_token(&pool, &parameters.subscription_token)
            .await
            .context("Failed to get the subscriber id from token")?;
    match id {
        None => return Err(ConfirmError::UnauthorizedError),
        Some(id) => confirm_subscriber(&pool, &id)
            .await
            .context("Failed to confirm the subscription")?,
    }
    Ok(HttpResponse::Ok().finish())
}

#[derive(thiserror::Error)]
pub enum ConfirmError {
    #[error("Failed to find a user matching given token")]
    UnauthorizedError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ConfirmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for ConfirmError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::UnauthorizedError => StatusCode::UNAUTHORIZED,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(
    name = "Get subscriber ID from validation token",
    skip(pool)
)]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
       WHERE subscription_token = $1",
        subscription_token
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool))]
pub async fn confirm_subscriber(
    pool: &PgPool,
    subscriber_id: &Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "Update subscriptions SET status = 'confirmed' WHERE id = $1 AND status = 'pending_confirmation'",
        subscriber_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
