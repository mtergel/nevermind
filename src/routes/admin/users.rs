use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app::{
        error::AppError,
        utils::{cursor_pagination::CPagination, types::Timestamptz},
        ApiContext,
    },
    routes::docs::ADMIN_TAG,
};

// Pagination types
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationInput {
    #[schema(value_type = Option<String>)]
    cursor: Option<CPagination>,
}

#[derive(Serialize, ToSchema)]
pub struct UserListResponse {
    data: Vec<UserData>,
    #[schema(value_type = Option<String>)]
    next_cursor: Option<CPagination>,
}

#[derive(Serialize, ToSchema)]
pub struct UserData {
    user_id: Uuid,
    username: String,
    #[schema(value_type = String)]
    created_at: Timestamptz,
}

#[utoipa::path(
    get,
    path = "/users",
    tag = ADMIN_TAG,
    security(
        ("bearerAuth" = ["user.view"])
    ),
    request_body = PaginationInput,
    responses(
        (status = 200, description = "List user, ordered by created at", body = UserListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden, scope not present"),
        (status = 500, description = "Internal server error")
    )

)]
#[tracing::instrument(name = "List users", skip_all, fields(req = ?req))]
pub async fn list_users(
    ctx: State<ApiContext>,
    Query(req): Query<PaginationInput>,
) -> Result<Json<UserListResponse>, AppError> {
    let page_size: i64 = 25;
    match req.cursor {
        Some(cursor) => {
            let next_res = sqlx::query_as!(
                UserData,
                r#"
                    select u.user_id, u.username, u.created_at
                    from "user" u
                    where (created_at, user_id) < ($1, $2)
                    order by created_at desc, user_id desc
                    limit $3
                "#,
                cursor.created_at.0,
                cursor.id,
                page_size
            )
            .fetch_all(&*ctx.db_pool)
            .await?;

            let next_cursor: Option<CPagination> = if next_res.len() < page_size.try_into().unwrap()
            {
                None
            } else {
                next_res.last().map(|item| CPagination {
                    id: item.user_id,
                    created_at: item.created_at.clone(),
                })
            };

            return Ok(Json(UserListResponse {
                data: next_res,
                next_cursor,
            }));
        }
        None => {
            // When there's no cursor, just fetch the first 25 rows
            let next_res = sqlx::query_as!(
                UserData,
                r#"
                    select u.user_id, u.username, u.created_at
                    from "user" u
                    order by created_at desc, user_id desc
                    limit $1
                "#,
                page_size
            )
            .fetch_all(&*ctx.db_pool)
            .await?;

            let next_cursor: Option<CPagination> = if next_res.len() < page_size.try_into().unwrap()
            {
                None
            } else {
                next_res.last().map(|item| CPagination {
                    id: item.user_id,
                    created_at: item.created_at.clone(),
                })
            };

            return Ok(Json(UserListResponse {
                data: next_res,
                next_cursor,
            }));
        }
    };
}
