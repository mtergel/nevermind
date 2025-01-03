use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use email::{add_email, delete_user_email, list_user_email, update_email_to_primary};
use me::{complete_me_profile, get_me_profile, update_me_profile};
use password::{change_password, forgot_password, reset_password};
use register::register_user;
use session::{list_active_sessions, revoke_session, revoke_session_by_id};
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
        .route("/auth/me", get(get_me_profile).patch(update_me_profile))
        .route("/auth/me/complete", post(complete_me_profile))
        .route("/auth/emails", post(add_email).get(list_user_email))
        .route("/auth/emails/:id", delete(delete_user_email))
        .route("/auth/emails/verify/:token", post(verify_email))
        .route("/auth/emails/resend", post(resend_email_verification))
        .route("/auth/emails/:id/primary", patch(update_email_to_primary))
        .route("/auth/change-password", post(change_password))
        .route("/auth/sessions", get(list_active_sessions))
        .route("/auth/sessions/revoke", delete(revoke_session))
}

// Called when the user is logging out from the Next.js server
pub fn api_key_protected() -> Router<ApiContext> {
    Router::new().route("/auth/sessions/:id/revoke", delete(revoke_session_by_id))
}

pub fn public_router() -> Router<ApiContext> {
    Router::new()
        .route("/auth/users", post(register_user))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
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
    password::change_password,
    me::get_me_profile,
    me::complete_me_profile,
    me::update_me_profile,
    session::list_active_sessions,
    session::revoke_session,
    session::revoke_session_by_id
))]
pub struct AuthApi;
