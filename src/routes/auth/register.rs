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
pub struct RegisterUserInput {
    #[validate(regex(path = *USERNAME_REGEX))]
    username: String,
    #[validate(email)]
    email: String,
    #[schema(value_type = String)]
    password: SecretString,
}

#[utoipa::path(
    post,
    path = "/users",
    tag = AUTH_TAG,
    request_body = RegisterUserInput,
    responses(
        (status = 201, description = "Successful created"),
        (status = 400, description = "Bad request"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Register user", skip_all)]
pub async fn register_user(
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<RegisterUserInput>,
) -> Result<(), AppError> {
    let password_hash = compute_password_hash(req.password).await?;
    let mut tx = ctx.db_pool.begin().await?;

    let user_id = sqlx::query_scalar!(
        r#"
            insert into "user" (username, password_hash, image)
            values ($1, $2, $3)
            returning user_id
        "#,
        req.username,
        password_hash,
        generate_avatar(&req.username)
    )
    .fetch_one(&mut *tx)
    .await
    .on_constraint("user_username_key", |_| {
        AppError::unprocessable_entity([("username", "taken")])
    })?;

    sqlx::query!(
        r#"
            insert into email (user_id, email, is_primary)
            values ($1, $2, true)
        "#,
        user_id,
        req.email,
    )
    .execute(&mut *tx)
    .await
    .on_constraint("email_email_key", |_| {
        AppError::unprocessable_entity([("email", "taken")])
    })?;

    let otp_manager = EmailVerifyOtp { user_id };
    let token = otp_manager.generate_otp();

    otp_manager
        .store_data(&token, &ctx.redis_client, &req.email)
        .await?;

    EmailVerifyOtp::send_email(&ctx.email_client, &token, &req.email).await?;

    // Store unverified user
    tx.commit().await?;

    Ok(())
}
