use axum::extract::State;
use secrecy::SecretString;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    app::{
        auth::password::compute_password_hash,
        error::{AppError, ResultExt},
        extrator::ValidatedJson,
        otp::{email_otp::EmailVerifyOtp, OtpManager},
        utils::{avatar_generator::generate_avatar, validation::USERNAME_REGEX},
        ApiContext,
    },
    routes::docs::AUTH_TAG,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ForgotPasswordInput {
    #[validate(email)]
    email: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordInput {
    token: String,
    #[schema(value_type = Option<String>)]
    new_password: SecretString,
}

#[utoipa::path(
    post,
    path = "/forgot-password",
    tag = AUTH_TAG,
    request_body = ForgotPasswordInput,
    responses(
        (status = 200, description = "Instruction sent"),
        (status = 400, description = "Bad request"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Forgot password", skip_all, fields(req = ?req))]
pub async fn forgot_password(
    _ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<ForgotPasswordInput>,
) -> Result<(), AppError> {
    // check email
    // generate otp
    // send otp

    Ok(())
}

#[utoipa::path(
    post,
    path = "/reset-password/{token}",
    tag = AUTH_TAG,
    request_body = ResetPasswordInput,
    responses(
        (status = 204, description = "Successfully updated the password"),
        (status = 400, description = "Bad request"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Reset password", skip_all, fields(req = ?req))]
pub async fn reset_password(
    _ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<ResetPasswordInput>,
) -> Result<(), AppError> {
    // check token
    // update password

    Ok(())
}
