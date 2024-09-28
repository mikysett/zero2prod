use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let app = spawn_app().await;

    let response = app.get_change_password().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &another_new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different passwords - \
        the field values must match.</i></p>"
    ))
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let wrong_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": wrong_password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(
        html_page.contains("<p><i>The current password is incorrect.</i></p>")
    )
}

#[tokio::test]
async fn new_password_must_be_at_least_12_characters() {
    let app = spawn_app().await;
    let short_password = "a".repeat(11);

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": &short_password,
            "new_password_check": &short_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>The new password must be at least 12 characters long.</i></p>"
    ))
}

#[tokio::test]
async fn new_password_must_be_at_most_128_characters() {
    let app = spawn_app().await;
    let short_password = "a".repeat(129);

    // Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": &short_password,
            "new_password_check": &short_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>The new password must be at most 128 characters long.</i></p>"
    ))
}

#[tokio::test]
async fn change_password() {
    let app = spawn_app().await;
    let new_password = "a".repeat(12);

    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    });

    // Login
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(
        html_page.contains(r#"<p><i>Your password has been changed.</i></p>"#)
    );

    // Logout
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Follow the redirect
    let html_page = app.get_login_html().await;
    assert!(
        html_page.contains(r#"<p><i>You have succesfully logged out.</i></p>"#)
    );

    // Login with the new password
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": new_password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");
}
