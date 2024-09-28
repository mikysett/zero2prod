use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

#[tracing::instrument(
    name = "Logout user from current session",
    skip(session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn log_out(
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        Ok(see_other("/login"))
    } else {
        session.logout();
        FlashMessage::info("You have succesfully logged out.").send();
        Ok(see_other("/login"))
    }
}
