use anyhow::Context;
use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    app::{
        error::AppError,
        extrator::{AuthUser, ValidatedJson},
        otp::{email_otp::EmailVerifyOtp, OtpManager},
        ApiContext,
    },
    config::Stage,
    routes::docs::EMAIL_TAG,
};

#[utoipa::path(
    post,
    path = "/emails/verify/{token}",
    tag = EMAIL_TAG,
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("token" = String, Path, description = "Token sent to user")
    ),
    responses(
        (status = 205, description = "Successful verified"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Verify email", skip_all)]
pub async fn verify_email(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    Path(token): Path<String>,
) -> Result<(), AppError> {
    let otp_manager = EmailVerifyOtp {
        user_id: auth_user.user_id,
        should_hash: ctx.config.stage == Stage::Prod,
    };

    let email_to_verify = otp_manager.get_data(&token, &ctx.redis_client).await?;

    match email_to_verify {
        Some(email) => {
            update_email_status_to_verified(&email, &ctx.db_pool).await?;

            Ok(())
        }
        None => Err(AppError::NotFound),
    }
}

#[tracing::instrument(name = "Updating email to verified", skip_all)]
async fn update_email_status_to_verified(email: &str, pool: &PgPool) -> anyhow::Result<()> {
    let _ = sqlx::query!(
        r#"
            update email 
            set verified = true, confirmation_sent_at = null
            where email = $1                
        "#,
        email
    )
    .execute(pool)
    .await
    .context("failed to set email to verified");

    Ok(())
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResendEmailVerificationInput {
    #[validate(email)]
    email: String,
}

#[utoipa::path(
    post,
    path = "/emails/resend",
    tag = EMAIL_TAG,
    security(
        ("bearerAuth" = [])
    ),
    request_body = ResendEmailVerificationInput,
    responses(
        (status = 204, description = "Successful sent email"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Resend email verification", skip_all, fields(req = ?req))]
pub async fn resend_email_verification(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<ResendEmailVerificationInput>,
) -> Result<StatusCode, AppError> {
    let mut tx = ctx.db_pool.begin().await?;

    let row = sqlx::query!(
        r#"
            select verified, confirmation_sent_at
            from email
            where email = $1 and user_id = $2
        "#,
        req.email,
        auth_user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;

    if row.verified {
        return Err(AppError::unprocessable_entity([("email", "verified")]));
    }

    let otp_manager = EmailVerifyOtp {
        user_id: auth_user.user_id,
        should_hash: ctx.config.stage == Stage::Prod,
    };

    let otps = otp_manager.get_keys(&ctx.redis_client, &req.email).await?;
    let token = if otps.is_empty() {
        let new_token = otp_manager.generate_otp();
        otp_manager
            .store_data(&new_token, &ctx.redis_client, &req.email)
            .await?;
        new_token
    } else {
        otps.first().unwrap().to_string()
    };

    EmailVerifyOtp::send_email(&ctx.email_client, &token, &req.email).await?;

    sqlx::query!(
        r#"
            update email
            set confirmation_sent_at = now()
            where email = $1 and user_id = $2
        "#,
        req.email,
        auth_user.user_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
