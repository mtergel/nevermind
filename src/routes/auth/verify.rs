use anyhow::Context;
use axum::extract::State;
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use validator::Validate;

use crate::app::{
    error::AppError,
    extrator::{AuthUser, ValidatedJson},
    otp::email_otp::EmailVerifyOtp,
    ApiContext,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyEmailInput {
    token: String,
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
        (status = 404, description = "Not found"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Verify email", skip_all)]
pub async fn verify_email(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<VerifyEmailInput>,
) -> Result<(), AppError> {
    let otp_manager = EmailVerifyOtp {
        user_id: auth_user.user_id,
    };

    let email_to_verify = otp_manager.get_data(&req.token, &ctx.redis_client).await?;

    match email_to_verify {
        Some(email) => {
            update_email_status_to_verified(&email, &ctx.db_pool).await?;

            Ok(())
        }
        None => Err(AppError::NotFound),
    }
}

async fn update_email_status_to_verified(email: &str, pool: &PgPool) -> anyhow::Result<()> {
    let _ = sqlx::query!(
        r#"
            update email 
            set verified = true
            where email = $1                
        "#,
        email
    )
    .execute(pool)
    .await
    .context("failed to set email to verified");

    Ok(())
}
