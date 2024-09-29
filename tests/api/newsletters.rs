use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{
    assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp,
};

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
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
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
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
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
                "content_text": "Newsletter body as plain text",
                "content_html": "<p>Newsletter body as HTML</p>",
            }),
            vec!["Field title can't be empty"],
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "content_text": "Newsletter body as plain text",
                "content_html": ""
            }),
            vec!["Field HTML content can't be empty"],
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "content_text": "",
                "content_html": "<p>Newsletter body as HTML</p>"
            }),
            vec!["Field text content can't be empty"],
        ),
        (
            serde_json::json!({
                "title": "",
                "content_text": "",
                "content_html": ""
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
        "content_text": "Newsletter body as plain text",
        "content_html": "<p>Newsletter body as HTML</p>",
        }))
        .await;

    assert_is_redirect_to(&response, "/login");
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

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
