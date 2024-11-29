use axum::extract::State;

use crate::{
    app::{error::AppError, extrator::AuthUser, ApiContext},
    routes::docs::ADMIN_TAG,
};

#[utoipa::path(
    get,
    path = "",
    tag = ADMIN_TAG,
    security(
        ("bearerAuth" = ["user.view"])
    ),
)]
#[tracing::instrument(name = "List users", skip_all)]
pub async fn list_users(_auth_user: AuthUser, _ctx: State<ApiContext>) -> Result<(), AppError> {
    Ok(())
}
