use axum::{extract::State, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    app::{error::AppError, extrator::AuthUser, ApiContext},
    routes::docs::AUTH_TAG,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    username: String,
    email: String,
    email_verified: bool,
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
            select u.user_id, u.username, e.email, e.verified
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
    }))
}
