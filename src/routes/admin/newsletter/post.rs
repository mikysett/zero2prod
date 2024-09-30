use actix_web::{
    web::{self, ReqData},
    HttpResponse,
};
use actix_web_flash_messages::FlashMessage;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::SubscriberEmail,
    email_client::EmailClient,
    idempotency::{get_saved_response, IdempotencyKey},
    utils::{e400, e500, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(form, pool, email_client),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    tracing::Span::current()
        .record("user_id", &tracing::field::display(user_id));

    if let Err(violation_messages) = validate_newsletter_issue(&form.0) {
        for message in violation_messages {
            FlashMessage::error(message).send();
        }
        return Ok(see_other("/admin/newsletters"));
    }

    // Destructure formData to avoid borrow checker issues
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey =
        idempotency_key.try_into().map_err(e400)?;

    if let Some(saved_response) =
        get_saved_response(&pool, idempotency_key, *user_id)
            .await
            .map_err(e500)?
    {
        FlashMessage::info("Newsletter sent successfully.").send();
        return Ok(saved_response);
    }

    let subscribers = match get_confirmed_subscribers(&pool).await {
        Ok(subscribers) => subscribers,
        Err(_) => {
            FlashMessage::error("Failed to get confirmed subscribers.").send();
            return Ok(see_other("/admin/newsletters"));
        }
    };

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                if let Err(_) = email_client
                    .send_email(
                        &subscriber.email,
                        &title,
                        &html_content,
                        &text_content,
                    )
                    .await
                {
                    FlashMessage::error(format!(
                        "Failed to send newsletter issue to {}",
                        subscriber.email
                    ))
                    .send();
                    return Ok(see_other("/admin/newsletters"));
                }
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid"
                )
            }
        }
    }

    FlashMessage::info("Newsletter sent successfully.").send();
    Ok(see_other("/admin/newsletters"))
}

fn validate_newsletter_issue(form: &FormData) -> Result<(), Vec<String>> {
    let violation_messages: Vec<String> = [
        ("title", &form.title),
        ("HTML content", &form.html_content),
        ("text content", &form.text_content),
    ]
    .iter()
    .filter_map(|field| {
        if field.1.is_empty() {
            Some(format!("Field {} can't be empty", field.0))
        } else {
            None
        }
    })
    .collect();

    if !violation_messages.is_empty() {
        return Err(violation_messages);
    }
    Ok(())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
pub async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sqlx::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
            SELECT email
            FROM subscriptions
            WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(e) => Err(anyhow::anyhow!(e)),
    })
    .collect();
    Ok(confirmed_subscribers)
}
