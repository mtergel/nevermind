use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app::{error::AppError, extrator::ExtractLocale, utils::types::Timestamptz, ApiContext},
    routes::docs::ADMIN_TAG,
};

#[derive(Serialize, ToSchema)]
pub struct BusinessResponse {
    business_id: Uuid,
    name: Option<String>,
    #[schema(value_type = String)]
    created_at: Timestamptz,
}

#[utoipa::path(
    get,
    path = "/business/{id}",
    tag = ADMIN_TAG,
    security(
        // TODO
        ("bearerAuth" = ["user.view"])
    ),
    params(
        ("id" = String, Path, description = "Business database id")
    ),
    responses(
        (status = 200, description = "Business detail", body = BusinessResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden, scope not present"),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Get business", skip_all)]
pub async fn get_business(
    ctx: State<ApiContext>,
    Path(id): Path<Uuid>,
    ExtractLocale(locale): ExtractLocale,
) -> Result<Json<BusinessResponse>, AppError> {
    let row = sqlx::query_as!(
        BusinessResponse,
        r#"
            select
                b.business_id,
                coalesce(nullif(b.name->$1, ''), b.name->'en') as name,
                b.created_at
            from business b
            where business_id = $2
        "#,
        locale,
        id
    )
    .fetch_one(&*ctx.db_pool)
    .await?;

    Ok(Json(row))
}
