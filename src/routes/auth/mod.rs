use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use email::{add_email, delete_user_email, list_user_email, update_email_to_primary};
use me::get_me_profile;
use password::{forgot_password, reset_password};
use register::register_user;
use session::{list_active_sessions, revoke_session};
use utoipa::OpenApi;
use verify::{resend_email_verification, verify_email};

use crate::app::ApiContext;

pub mod email;
pub mod me;
pub mod password;
pub mod register;
pub mod session;
pub mod verify;

pub fn router() -> Router<ApiContext> {
    Router::new()
        .route("/auth/me", get(get_me_profile))
        .route("/auth/users", post(register_user))
        .route("/auth/emails", post(add_email).get(list_user_email))
        .route("/auth/emails/:id", delete(delete_user_email))
        .route("/auth/emails/verify/:token", post(verify_email))
        .route("/auth/emails/resend", post(resend_email_verification))
        .route("/auth/emails/:id/primary", patch(update_email_to_primary))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
        .route("/auth/sessions", get(list_active_sessions))
        .route("/auth/sessions/revoke", delete(revoke_session))
}

#[derive(OpenApi)]
#[openapi(paths(
    register::register_user,
    email::add_email,
    email::list_user_email,
    email::delete_user_email,
    verify::verify_email,
    verify::resend_email_verification,
    email::update_email_to_primary,
    password::forgot_password,
    password::reset_password,
    me::get_me_profile,
    session::list_active_sessions,
    session::revoke_session
))]
pub struct AuthApi;
