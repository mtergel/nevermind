use axum::{extract::State, http::StatusCode, Json};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    app::{
        auth::password::compute_password_hash,
        error::{AppError, ResultExt},
        extrator::{AuthUser, ValidatedJson},
        utils::validation::USERNAME_REGEX,
        ApiContext,
    },
    routes::docs::AUTH_TAG,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    username: String,
    email: String,
    email_verified: bool,
    bio: String,
    image: Option<String>,
    reset_password: Option<bool>,
    reset_username: Option<bool>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CompleteMeInput {
    #[validate(regex(path = *USERNAME_REGEX))]
    username: Option<String>,
    #[schema(value_type = Option<String>)]
    password: Option<SecretString>,
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize, Validate, ToSchema)]
#[serde(default)] // fill in any missing fields with `..UpdateUser::default()`
pub struct UpdateUserInput {
    bio: Option<String>,
    image: Option<String>,
}

#[utoipa::path(
    get,
    path = "/me",
    tag = AUTH_TAG,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Successful created", body = MeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Get me profile", skip_all)]
pub async fn get_me_profile(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
) -> Result<Json<MeResponse>, AppError> {
    let res = sqlx::query!(
        r#"
            select u.user_id, u.username, e.email, e.verified,
            u.bio, u.image, u.reset_username, u.reset_password
            from email e
            inner join "user" u using (user_id)
            where e.user_id = $1 and e.is_primary = true
        "#,
        auth_user.user_id
    )
    .fetch_one(&*ctx.db_pool)
    .await?;

    Ok(Json(MeResponse {
        username: res.username,
        email: res.email,
        email_verified: res.verified,
        bio: res.bio,
        image: res.image,
        reset_password: res.reset_password,
        reset_username: res.reset_username,
    }))
}

#[utoipa::path(
    post,
    path = "/me/complete",
    tag = AUTH_TAG,
    request_body = CompleteMeInput,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 204, description = "Successful completed profile"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Complete me profile", skip_all)]
pub async fn complete_me_profile(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<CompleteMeInput>,
) -> Result<StatusCode, AppError> {
    let mut tx = ctx.db_pool.begin().await?;
    let user_metadata = sqlx::query!(
        r#"
            select reset_username, reset_password
            from "user"
            where user_id = $1
        "#,
        auth_user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;

    if req.username.is_some() && Some(true) == user_metadata.reset_username {
        let _ = sqlx::query!(
            r#"
                update "user"
                set username = $1, reset_username = null
                where user_id = $2
            "#,
            req.username,
            auth_user.user_id
        )
        .execute(&mut *tx)
        .await
        .on_constraint("user_username_key", |_| {
            AppError::unprocessable_entity([("username", "taken")])
        })?;
    }

    if req.password.is_some() && Some(true) == user_metadata.reset_password {
        let password_hash = compute_password_hash(req.password.unwrap()).await?;
        let _ = sqlx::query!(
            r#"
                update "user"
                set password_hash = $1, reset_password = null
                where user_id = $2
            "#,
            password_hash,
            auth_user.user_id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    patch,
    path = "/me",
    tag = AUTH_TAG,
    request_body = UpdateUserInput,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 204, description = "Successful updated profile"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Update me profile", skip_all)]
pub async fn update_me_profile(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<UpdateUserInput>,
) -> Result<StatusCode, AppError> {
    if req == UpdateUserInput::default() {
        return Ok(StatusCode::NO_CONTENT);
    }

    let _ = sqlx::query!(
        r#"
            update "user"
            set bio = coalesce($1, "user".bio),
                image = coalesce($2, "user".image)
            where user_id = $3
        "#,
        req.bio,
        req.image,
        auth_user.user_id
    )
    .execute(&*ctx.db_pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
