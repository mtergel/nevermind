use axum::{routing::post, Router};
use email::{add_email, update_email_to_primary};
use register::register_user;
use utoipa::OpenApi;
use verify::verify_email;

use crate::app::ApiContext;

pub mod email;
pub mod register;
pub mod verify;

pub fn router() -> Router<ApiContext> {
    Router::new()
        .route("/auth/users", post(register_user))
        .route("/auth/emails", post(add_email))
        .route("/auth/emails/verify", post(verify_email))
        .route("/auth/emails/primary", post(update_email_to_primary))
}

#[derive(OpenApi)]
#[openapi(paths(
    register::register_user,
    verify::verify_email,
    email::add_email,
    email::update_email_to_primary
))]
pub struct AuthApi;
