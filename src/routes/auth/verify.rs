use axum::{extract::State, routing::post, Router};
use secrecy::SecretString;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

use crate::app::{
    error::AppError,
    extrator::{AuthUser, ValidatedJson},
    otp::{email_otp::EmailVerifyOtp, OtpManager},
    ApiContext,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyEmailInput {
    #[schema(value_type = String)]
    token: SecretString,
}

#[utoipa::path(
    post,
    path = "/verify",
    security(
        ("bearerAuth" = [])
    ),
    request_body = VerifyEmailInput,
    responses(
        (status = 200, description = "Successful created"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Register user", skip_all)]
pub async fn verify_email(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<VerifyEmailInput>,
) -> Result<(), AppError> {
    Ok(())
}
