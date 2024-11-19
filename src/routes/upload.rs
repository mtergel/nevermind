use std::{collections::HashMap, str::FromStr};

use axum::{extract::State, routing::post, Json, Router};
use mime2::Mime;
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use validator::{Validate, ValidateArgs, ValidationError};

use crate::app::{error::AppError, extrator::AuthUser, storage::path::S3Path, ApiContext};

use super::docs::UPLOAD_TAG;

#[derive(OpenApi)]
#[openapi(paths(handle_upload))]
pub struct UploadApi;

#[derive(Debug, Validate, Deserialize, ToSchema)]
#[validate(context = S3Path)]
pub struct UploadFile {
    pub path: S3Path,
    pub file_name: String,
    #[validate(custom(function = "validate_mime_type", use_context))]
    pub file_type: String,

    #[validate(custom(function = "validate_file_size", use_context))]
    pub file_size: i64,
}

#[derive(Serialize, ToSchema)]
pub struct PresignedResult {
    uri: String,
    method: String,
    headers: HashMap<String, String>,
}

pub fn router() -> Router<ApiContext> {
    Router::new().route("/upload", post(handle_upload))
}

#[utoipa::path(
    post,
    path = "",
    tag = UPLOAD_TAG,
    security(
        ("bearerAuth" = [])
    ),
    request_body = UploadFile,
    responses(
        (status = 200, description = "Successful created presigned result", body = PresignedResult),
        (status = 400, description = "Bad request"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Generate presigned", skip_all, fields(req = ?req))]
async fn handle_upload(
    auth_user: AuthUser,
    ctx: State<ApiContext>,
    Json(req): Json<UploadFile>,
) -> Result<Json<PresignedResult>, AppError> {
    req.validate_with_args(&req.path)?;

    let request = ctx
        .storage_client
        .generate_upload_presigned_url(
            format!("{}/{}/{}", req.path, auth_user.user_id, req.file_name),
            req.file_type,
            req.file_size,
        )
        .await?;

    let headers: HashMap<String, String> = request
        .headers()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    Ok(Json(PresignedResult {
        uri: request.uri().to_string(),
        method: request.method().to_string(),
        headers,
    }))
}

fn validate_file_size(file_size: i64, path: &S3Path) -> Result<(), ValidationError> {
    if file_size > path.get_max_size() {
        return Err(ValidationError::new("file_size"));
    }

    Ok(())
}

fn validate_mime_type(file_type: &str, path: &S3Path) -> Result<(), ValidationError> {
    let mime_type = Mime::from_str(file_type).map_err(|_| ValidationError::new("file_type"))?;

    if !path.is_allowed_type(&mime_type) {
        return Err(ValidationError::new("file_type"));
    }

    Ok(())
}
