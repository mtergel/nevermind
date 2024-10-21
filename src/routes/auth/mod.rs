use axum::{
    routing::{delete, patch, post},
    Router,
};
use email::{add_email, delete_user_email, list_user_email, update_email_to_primary};
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
        .route("/auth/emails", post(add_email).get(list_user_email))
        .route("/auth/emails/:id", delete(delete_user_email))
        .route("/auth/emails/verify/:token", post(verify_email))
        .route("/auth/emails/:id/primary", patch(update_email_to_primary))
}

#[derive(OpenApi)]
#[openapi(paths(
    register::register_user,
    email::add_email,
    email::list_user_email,
    email::delete_user_email,
    verify::verify_email,
    email::update_email_to_primary
))]
pub struct AuthApi;
