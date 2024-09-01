use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::HttpResponse;
use chrono::Utc;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use crate::domain::NewSubscriber;
use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;
use crate::domain::SubscriptionToken;
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;

#[derive(serde::Deserialize)]
pub struct FormData {
    #[allow(dead_code)]
    email: String,
    #[allow(dead_code)]
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let mut transaction = pool.begin().await.map_err(|e| {
        SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to acquire a Postgres connection from the pool".into(),
        )
    })?;
    let new_subscriber =
        form.0.try_into().map_err(SubscribeError::ValidationError)?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .map_err(|e| {
            SubscribeError::UnexpectedError(
                Box::new(e),
                "Failed to insert new subscriber in the database".into(),
            )
        })?;
    let subscription_token = store_token(&mut transaction, subscriber_id)
        .await
        .map_err(|e| {
            SubscribeError::UnexpectedError(
                Box::new(e),
                "Failed to store the confirmation token for a new subscriber"
                    .into(),
            )
        })?;
    transaction.commit().await.map_err(|e| {
        SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to send a confirmation email".into(),
        )
    })?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .map_err(|e| {
        SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to send confirmation email".into(),
        )
    })?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error("{1}")]
    UnexpectedError(#[source] Box<dyn std::error::Error>, String),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_, _) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
) -> Result<SubscriptionToken, StoreTokenError> {
    let new_subscription_token =
        SubscriptionToken::generate_subscription_token();
    let subscription_token = sqlx::query!(
        r#"
        WITH existing_user AS (
            SELECT subscription_token FROM subscription_tokens WHERE subscriber_id = $2
        ), insert_if_needed AS (
            INSERT INTO subscription_tokens (subscription_token, subscriber_id)
            SELECT $1, $2
            WHERE NOT EXISTS (SELECT 1 FROM existing_user)
            RETURNING subscription_token
        )
        SELECT subscription_token FROM insert_if_needed
        UNION ALL
        SELECT subscription_token FROM existing_user
        LIMIT 1;
        "#,
        new_subscription_token.as_ref(),
        subscriber_id,
    )
    .fetch_one(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;

    match subscription_token.subscription_token {
        Some(raw_subscription_token) => {
            SubscriptionToken::parse(raw_subscription_token)
                .map_err(|e| StoreTokenError(sqlx::Error::Decode(e.into())))
        }
        None => Err(StoreTokenError(sqlx::Error::RowNotFound)),
    }
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source()
    }
    Ok(())
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A database error was encoutered while trying to store a subscription token")
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[tracing::instrument(
    name = "Saving new subscriber in the database",
    skip(transaction, new_subscriber)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let subscriber_id = sqlx::query!(
        r#"
        WITH insert_or_select AS (
            INSERT INTO subscriptions (id, email, name, subscribed_at, status)
            SELECT $1, $2, $3, $4, 'pending_confirmation'
            ON CONFLICT (email) DO NOTHING
            RETURNING id
        )
        SELECT id FROM insert_or_select
        UNION ALL
        SELECT id FROM subscriptions WHERE email = $2
        LIMIT 1;
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .fetch_one(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    subscriber_id
        .id
        .ok_or(sqlx::Error::ColumnNotFound("id".to_string()))
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &SubscriptionToken,
) -> Result<(), reqwest::Error> {
    let confirmation_link = &format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url,
        subscription_token.as_ref(),
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let text_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(new_subscriber.email, "Welcome", &html_body, &text_body)
        .await
}
