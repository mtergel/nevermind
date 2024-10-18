use axum::{routing::post, Router};
use register::register_user;
use utoipa::OpenApi;
use verify::verify_email;

use crate::app::ApiContext;

pub mod register;
pub mod verify;

pub fn router() -> Router<ApiContext> {
    Router::new()
        .route("/auth/register", post(register_user))
        .route("/auth/verify", post(verify_email))
}

#[derive(OpenApi)]
#[openapi(paths(register::register_user, verify::verify_email))]
pub struct AuthApi;
