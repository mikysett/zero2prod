use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};
use actix_web::{
    body::{BoxBody, EitherBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
};

pub async fn auth_guard(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<
    ServiceResponse<EitherBody<BoxBody, impl MessageBody>>,
    actix_web::Error,
> {
    // Extract the session from the request
    let session = req.extract::<TypedSession>().await.map_err(e500)?;

    // pre-processing - Check that a username exists for the given user_id in the session
    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(req.into_response(see_other("/login").map_into_left_body()));
    }

    // invoke the wrapped middleware or service
    let res = next.call(req).await?;

    // post-processing

    Ok(res.map_into_right_body())
}