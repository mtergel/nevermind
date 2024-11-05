use axum::{extract::State, Json};

use crate::{
    app::{
        auth::session::{Session, SessionData},
        error::AppError,
        extrator::AuthUser,
        ApiContext,
    },
    routes::docs::SESSION_TAG,
};

#[utoipa::path(
    get,
    path= "/sessions",
    tag = SESSION_TAG,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Successful", body = Vec<SessionData>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_active_sessions(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
) -> Result<Json<Vec<SessionData>>, AppError> {
    let session = Session {
        user_id: auth_user.user_id,
        session_id: auth_user.session_id,
    };

    let sessions = session.list_sessions(&ctx.redis_client).await?;
    Ok(Json(sessions))
}
