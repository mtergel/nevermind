use axum::{extract::State, http::StatusCode};
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    app::{
        auth::password::compute_password_hash,
        email::client::EmailClient,
        error::AppError,
        extrator::{AuthUser, ValidatedJson},
        otp::{email_forgot_otp::EmailForgotOtp, OtpManager},
        utils::validation::validate_password,
        ApiContext,
    },
    config::Stage,
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
    #[schema(value_type = String)]
    new_password: SecretString,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordInput {
    #[schema(value_type = String)]
    password: SecretString,
    #[schema(value_type = String)]
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
        (status = 404, description = "User not found"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Forgot password", skip_all, fields(req = ?req))]
pub async fn forgot_password(
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<ForgotPasswordInput>,
) -> Result<(), AppError> {
    let email_ok = check_email(&req.email, &ctx.db_pool).await?;
    if !email_ok {
        return Ok(());
    }

    // generate otp
    let otp_manager = EmailForgotOtp {
        should_hash: ctx.config.stage == Stage::Prod,
    };
    let token = otp_manager.generate_otp();
    otp_manager
        .store_data(&token, &ctx.redis_client, &req.email)
        .await?;

    EmailForgotOtp::send_email(&ctx.email_client, &token, &req.email).await?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/reset-password",
    tag = AUTH_TAG,
    request_body = ResetPasswordInput,
    responses(
        (status = 204, description = "Successfully updated the password"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Token expired/missing"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Reset password", skip_all, fields(req = ?req))]
pub async fn reset_password(
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<ResetPasswordInput>,
) -> Result<StatusCode, AppError> {
    let otp_manager = EmailForgotOtp {
        should_hash: ctx.config.stage == Stage::Prod,
    };

    match otp_manager.get_data(&req.token, &ctx.redis_client).await? {
        Some(email) => {
            let password_hash = compute_password_hash(req.new_password).await?;
            let user_id = reset_user_password(&password_hash, &email, &ctx.db_pool).await?;

            send_password_notification_email(&user_id, &ctx.db_pool, &ctx.email_client, &email)
                .await;

            Ok(StatusCode::NO_CONTENT)
        }

        None => return Err(AppError::NotFound),
    }
}

async fn check_email(email: &str, pool: &PgPool) -> Result<bool, AppError> {
    let row = sqlx::query_scalar!(
        r#"
            select verified 
            from email
            where email = $1
        "#,
        email
    )
    .fetch_one(pool)
    .await;

    match row {
        Ok(v) => Ok(v),
        Err(err) => match err {
            sqlx::Error::RowNotFound => Ok(false),
            e => Err(AppError::from(e)),
        },
    }
}

#[tracing::instrument(name = "Updating password using email", skip_all)]
async fn reset_user_password(hash: &str, email: &str, pool: &PgPool) -> Result<Uuid, AppError> {
    let mut tx = pool.begin().await?;

    let user_id = sqlx::query_scalar!(
        r#"
            select user_id
            from email
            where email = $1
        "#,
        email
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
            update "user"
            set password_hash = $1
            where user_id = $2
        "#,
        hash,
        user_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(user_id)
}

#[utoipa::path(
    post,
    path = "/change-password",
    tag = AUTH_TAG,
    request_body = ChangePasswordInput,
    responses(
        (status = 204, description = "Successfully updated the password"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Wrong password"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Change password", skip_all, fields(req = ?req))]
pub async fn change_password(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<ChangePasswordInput>,
) -> Result<StatusCode, AppError> {
    validate_password(req.password, &auth_user.user_id, &ctx.db_pool).await?;
    let password_hash = compute_password_hash(req.new_password).await?;

    let _ = sqlx::query!(
        r#"
            update "user"
            set password_hash = $1
            where user_id = $2
        "#,
        password_hash,
        auth_user.user_id
    )
    .execute(&*ctx.db_pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[tracing::instrument(name = "Sending password changed notification", skip_all)]
async fn send_password_notification_email(
    user_id: &Uuid,
    pool: &PgPool,
    email_client: &EmailClient,
    cause_email: &str,
) {
    if let Ok(primary_email) = sqlx::query_scalar!(
        r#"
            select email
            from email
            where user_id = $1 and is_primary = true
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    {
        if let Ok(email_content) = email_client.build_password_changed(cause_email).await {
            let _ = email_client.send_email(&primary_email, email_content).await;
        }
    }
}
