use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

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
#[tracing::instrument(name = "List active sessions", skip_all)]
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

#[utoipa::path(
    delete,
    path= "/sessions/{id}/revoke",
    tag = SESSION_TAG,
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("id" = String, Path, description = "Session id")
    ),
    responses(
        (status = 204, description = "Successful"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Revoke session", skip_all, fields(id = ?id))]
pub async fn revoke_session(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let session = Session {
        user_id: auth_user.user_id,
        session_id: id,
    };

    session.revoke(&ctx.redis_client).await?;

    Ok(())
}
