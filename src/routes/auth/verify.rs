use anyhow::Context;
use axum::extract::{Path, State};
use sqlx::PgPool;

use crate::{
    app::{error::AppError, extrator::AuthUser, otp::email_otp::EmailVerifyOtp, ApiContext},
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
