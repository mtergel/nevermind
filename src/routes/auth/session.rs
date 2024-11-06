use axum::{
    extract::{Path, State},
    Json,
};
use secrecy::SecretString;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    app::{
        auth::session::{Session, SessionData},
        error::AppError,
        extrator::{ApiKey, AuthUser, ValidatedJson},
        utils::validation::validate_password,
        ApiContext,
    },
    routes::docs::SESSION_TAG,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RevokeSessionInput {
    session_id: Uuid,
    #[schema(value_type = String)]
    password: SecretString,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RevokeSessionByIdInput {
    user_id: Uuid,
}

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
    path= "/sessions/revoke",
    tag = SESSION_TAG,
    security(
        ("bearerAuth" = [])
    ),
    request_body = RevokeSessionInput,
    responses(
        (status = 204, description = "Successful"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Revoke session", skip_all, fields(req = ?req))]
pub async fn revoke_session(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<RevokeSessionInput>,
) -> Result<(), AppError> {
    validate_password(req.password, &auth_user.user_id, &ctx.db_pool).await?;

    let session = Session {
        user_id: auth_user.user_id,
        session_id: req.session_id,
    };

    session.revoke(&ctx.redis_client).await?;

    Ok(())
}

#[utoipa::path(
    delete,
    path= "/sessions/{id}/revoke",
    tag = SESSION_TAG,
    security(
        ("apiKeyAuth" = [])
    ),
    params(
        ("id" = String, Path, description = "Session id")
    ),
    request_body = RevokeSessionByIdInput,
    responses(
        (status = 204, description = "Successful"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Revoke session by id", skip_all, fields(req = ?req))]
pub async fn revoke_session_by_id(
    _api_key: ApiKey,
    ctx: State<ApiContext>,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<RevokeSessionByIdInput>,
) -> Result<(), AppError> {
    let session = Session {
        user_id: req.user_id,
        session_id: id,
    };

    session.revoke(&ctx.redis_client).await?;

    Ok(())
}
