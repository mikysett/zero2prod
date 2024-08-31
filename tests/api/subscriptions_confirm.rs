use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;
use zero2prod::domain::SubscriptionToken;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response =
        reqwest::get(&format!("{}/subscriptions/confirm", app.address))
            .await
            .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();
    assert_eq!(response.status().as_u16(), 200)
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn two_emails_are_sent_if_the_user_subscribes_twice() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_twice_do_not_return_errors() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    reqwest::get(confirmation_links.html.clone())
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    reqwest::get(confirmation_links.html.clone())
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn confirm_returns_a_400_when_token_do_not_exist() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!(
        "{}/subscriptions/confirm?subscirption_token={:?}",
        app.address,
        SubscriptionToken::generate_subscription_token()
    ))
    .await
    .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn confirm_do_not_change_the_status_if_not_pending_confirmation() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'another status'
        WHERE email = 'ursula_le_guin@gmail.com'
        "#
    )
    .execute(&app.db_pool)
    .await
    .expect("Failed to change subscriber status");

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);
    let response = reqwest::get(confirmation_links.html).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("SELECT status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.status, "another status");
}
