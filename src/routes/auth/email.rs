use axum::{extract::State, Json};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    app::{
        error::{AppError, ResultExt},
        extrator::{AuthUser, ValidatedJson},
        otp::{email_otp::EmailVerifyOtp, OtpManager},
        utils::{types::Timestamptz, validation::validate_password},
        ApiContext,
    },
    routes::docs::EMAIL_TAG,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AddEmailInput {
    #[validate(email)]
    new_email: String,
    #[schema(value_type = String)]
    password: SecretString,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateEmailToPrimaryInput {
    #[validate(email)]
    email: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct Email {
    email_id: String,
    email: String,
    verified: bool,
    is_primary: bool,
    #[schema(value_type = String, format = DateTime)]
    created_at: Timestamptz,
}

#[utoipa::path(
    post,
    path = "/emails",
    tag = EMAIL_TAG,
    security(
        ("bearerAuth" = [])
    ),
    request_body = AddEmailInput,
    responses(
        (status = 201, description = "Successful created"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Add email", skip_all, fields(req = ?req))]
pub async fn add_email(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<AddEmailInput>,
) -> Result<(), AppError> {
    validate_password(req.password, &auth_user.user_id, &ctx.db_pool).await?;

    let mut tx = ctx.db_pool.begin().await?;
    sqlx::query!(
        r#"
            insert into email (email, user_id)
            values ($1, $2)
        "#,
        req.new_email,
        auth_user.user_id
    )
    .execute(&mut *tx)
    .await
    .on_constraint("email_email_key", |_| {
        AppError::unprocessable_entity([("email", "taken")])
    })?;

    let otp_manager = EmailVerifyOtp {
        user_id: auth_user.user_id,
    };

    let token = otp_manager.generate_otp();

    otp_manager
        .store_data(&token, &ctx.redis_client, &req.new_email)
        .await?;

    EmailVerifyOtp::send_email(&ctx.email_client, &token, &req.new_email).await?;

    // Store unverified user
    tx.commit().await?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/emails/primary",
    tag = EMAIL_TAG,
    security(
        ("bearerAuth" = [])
    ),
    request_body = UpdateEmailToPrimaryInput,
    responses(
        (status = 200, description = "Successful updated"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Update email to primary", skip_all, fields(req = ?req, auth_user = ?auth_user))]
pub async fn update_email_to_primary(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<UpdateEmailToPrimaryInput>,
) -> Result<(), AppError> {
    let _ = sqlx::query!(
        r#"
            update email 
            set is_primary = true
            where email = $1 and user_id = $2
        "#,
        req.email,
        auth_user.user_id
    )
    .execute(&*ctx.db_pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::NotFound,

        err => AppError::Anyhow(err.into()),
    })?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/emails",
    tag = EMAIL_TAG,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Successful", body = Vec<Email>),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "List users emails", skip_all, , fields(auth_user = ?auth_user))]
pub async fn list_user_email(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
) -> Result<Json<Vec<Email>>, AppError> {
    let rows = sqlx::query_as!(
        Email,
        r#"
            select email_id, email, verified, is_primary, created_at 
            from email
            where user_id = $1
            limit 20
        "#,
        auth_user.user_id
    )
    .fetch_all(&*ctx.db_pool)
    .await
    .unwrap_or_default();

    Ok(Json(rows))
}
