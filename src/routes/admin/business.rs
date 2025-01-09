use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    app::{
        error::AppError,
        extrator::ValidatedJson,
        utils::{
            types::Timestamptz,
            validation::{BUSINESS_NAME_EN_REGEX, BUSINESS_NAME_MN_REGEX},
        },
        ApiContext,
    },
    routes::docs::ADMIN_TAG,
};

#[derive(Serialize, ToSchema)]
pub struct BusinessListResponse {
    data: Vec<BusinessData>,
}

#[derive(Serialize, ToSchema)]
pub struct BusinessData {
    business_id: Uuid,
    name: Option<String>,
    #[schema(value_type = String)]
    created_at: Timestamptz,
}

#[utoipa::path(
    get,
    path = "/business",
    tag = ADMIN_TAG,
    security(
        // TODO
        ("bearerAuth" = ["user.view"])
    ),
    responses(
        (status = 200, description = "List business, ordered by created at", body = BusinessListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden, scope not present"),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "List business", skip_all)]
pub async fn list_business(ctx: State<ApiContext>) -> Result<Json<BusinessListResponse>, AppError> {
    let rows = sqlx::query_as!(
        BusinessData,
        r#"
            select
                b.business_id,
                coalesce(nullif(b.name->$1, ''), b.name->'en') as name,
                b.created_at
            from business b
        "#,
        "mn"
    )
    // TODO: Pagination
    .fetch_all(&*ctx.db_pool)
    .await?;

    Ok(Json(BusinessListResponse { data: rows }))
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateBusinessInput {
    #[validate(regex(path = *BUSINESS_NAME_EN_REGEX))]
    name: String,

    #[validate(regex(path = *BUSINESS_NAME_MN_REGEX))]
    name_mn: Option<String>,
}

#[utoipa::path(
    post,
    path = "/business",
    tag = ADMIN_TAG,
    request_body = CreateBusinessInput,
    security(
        // TODO
        ("bearerAuth" = ["user.view"])
    ),
    responses(
        (status = 201, description = "Successfully created"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden, scope not present"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Create business", skip_all, fields(req = ?req))]
pub async fn create_business(
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<CreateBusinessInput>,
) -> Result<(), AppError> {
    Ok(())
}
