use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

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
) -> HttpResponse {
    let id = match get_subscriber_id_from_token(
        &pool,
        &parameters.subscription_token,
    )
    .await
    {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match id {
        None => HttpResponse::Unauthorized().finish(),
        Some(id) => {
            if confirm_subscriber(&pool, &id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
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
