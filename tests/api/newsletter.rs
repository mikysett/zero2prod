use std::time::Duration;

use fake::{
    faker::{internet::en::SafeEmail, name::en::Name},
    Fake,
};
use wiremock::{
    matchers::{any, method, path},
    Mock, MockBuilder, ResponseTemplate,
};

use crate::helpers::{
    assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp,
};

fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(
        html_page.contains(r#"<p><i>Newsletter sent successfully.</i></p>"#)
    );
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(
        html_page.contains(r#"<p><i>Newsletter sent successfully.</i></p>"#)
    );
}

#[tokio::test]
async fn newsletters_fields_must_not_be_empty() {
    let app = spawn_app().await;

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    let test_cases = vec![
        (
            serde_json::json!({
                "title": "",
                "text_content": "Newsletter body as plain text",
                "html_content": "<p>Newsletter body as HTML</p>",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            vec!["Field title can't be empty"],
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "text_content": "Newsletter body as plain text",
                "html_content": "",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            vec!["Field HTML content can't be empty"],
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "text_content": "",
                "html_content": "<p>Newsletter body as HTML</p>",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            vec!["Field text content can't be empty"],
        ),
        (
            serde_json::json!({
                "title": "",
                "text_content": "",
                "html_content": "",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            vec![
                "Field title can't be empty",
                "Field HTML content can't be empty",
                "Field text content can't be empty",
            ],
        ),
    ];

    for (invalid_body, error_messages) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;
        assert_is_redirect_to(&response, "/admin/newsletters");

        // Follow the redirect
        let html_page = app.get_newsletters_html().await;
        for message in error_messages {
            assert!(html_page.contains(&format!("<p><i>{}</i></p>", message)))
        }
    }
}

#[tokio::test]
async fn you_must_be_logged_in_to_send_newsletters() {
    let app = spawn_app().await;

    let response = app
        .post_newsletters(&serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;

    assert_is_redirect_to(&response, "/login");
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name": name,
        "email": email,
    }))
    .unwrap();

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(
        html_page.contains(r#"<p><i>Newsletter sent successfully.</i></p>"#)
    );

    // Send the newsletter **agian**
    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(
        html_page.contains(r#"<p><i>Newsletter sent successfully.</i></p>"#)
    );
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_delay(Duration::from_secs(2)),
        )
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response1 = app.post_newsletters(&newsletter_request_body);
    let response2 = app.post_newsletters(&newsletter_request_body);

    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
}

#[tokio::test]
async fn transient_errors_do_not_cause_duplicate_deliveries_on_retries() {
    let app = spawn_app().await;
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    create_confirmed_subscriber(&app).await;
    create_confirmed_subscriber(&app).await;

    app.test_user.login(&app).await;

    // Email delivery fails for the second subscriber
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 500);

    // Retry submitting the form will succeed for both subscribers now
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .named("Delivery retry")
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 303);
}
