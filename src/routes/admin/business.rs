use axum::{extract::State, Json};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app::{error::AppError, utils::types::Timestamptz, ApiContext},
    routes::docs::ADMIN_TAG,
};

#[derive(Serialize, ToSchema)]
pub struct BusinessListResponse {
    data: Vec<BusinessData>,
}

#[derive(Serialize, ToSchema)]
pub struct BusinessData {
    business_id: Uuid,
    name: String,
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
