use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    domain::{Password, PasswordError},
    session_state::TypedSession,
    utils::{e500, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    session: TypedSession,
    pg_pool: web::Data<PgPool>,
    form: web::Form<FormData>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = match session.get_username().map_err(e500)? {
        Some(username) => username,
        None => return Ok(see_other("/login")),
    };

    if form.0.new_password.expose_secret()
        != form.0.new_password_check.expose_secret()
    {
        FlashMessage::error("You entered two different passwords - the field values must match.").send();
        return Ok(see_other("/admin/password"));
    }

    let new_password =
        match Password::parse(form.0.new_password.expose_secret().clone()) {
            Ok(password) => password,
            Err(PasswordError::TooShort) => {
                FlashMessage::error(
                    "The new password must be at least 12 characters long.",
                )
                .send();
                return Ok(see_other("/admin/password"));
            }
            Err(PasswordError::TooLong) => {
                FlashMessage::error(
                    "The new password must be at most 128 characters long.",
                )
                .send();
                return Ok(see_other("/admin/password"));
            }
        };

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };
    if let Err(e) = validate_credentials(credentials, &pg_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.")
                    .send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    };
    todo!()
}