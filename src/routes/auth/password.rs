use axum::extract::State;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    app::{
        auth::password::compute_password_hash,
        error::AppError,
        extrator::ValidatedJson,
        otp::{email_forgot_otp::EmailForgotOtp, OtpManager},
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
) -> Result<(), AppError> {
    let otp_manager = EmailForgotOtp {
        should_hash: ctx.config.stage == Stage::Prod,
    };

    match otp_manager.get_data(&req.token, &ctx.redis_client).await? {
        Some(email) => {
            let password_hash = compute_password_hash(req.new_password).await?;
            let mut tx = ctx.db_pool.begin().await?;
            let user_id = sqlx::query_scalar!(
                r#"
                    select user_id
                    from email
                    inner join "user" using (user_id)
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
                password_hash,
                user_id
            )
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            let primary_email = sqlx::query_scalar!(
                r#"
                    select email
                    from email
                    where user_id = $1 and is_primary = true
                "#,
                user_id
            )
            .fetch_one(&*ctx.db_pool)
            .await?;

            let email_content = ctx.email_client.build_password_changed(&email).await?;
            ctx.email_client
                .send_email(&primary_email, email_content)
                .await?;

            Ok(())
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
