use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Postgres, QueryBuilder};
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

#[derive(Serialize, ToSchema, FromRow)]
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
    let page_size: usize = 25;
    let cursor_size: i64 = (page_size + 1) as i64;

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            select u.user_id, u.username, u.created_at 
            from "user" u
        "#,
    );

    if let Some(c) = req.cursor {
        query_builder.push(" where (created_at, user_id) <= (");
        let mut separated = query_builder.separated(", ");
        separated.push_bind(c.created_at);
        separated.push_bind(c.id);
        separated.push_unseparated(") ");
    }

    query_builder.push(" order by u.created_at desc, u.user_id desc ");

    query_builder.push(" limit ");
    query_builder.push_bind(cursor_size);

    let query = query_builder.build_query_as::<UserData>();
    let mut next_res = query.fetch_all(&*ctx.db_pool).await?;

    let next_cursor: Option<CPagination> = if next_res.len() < cursor_size.try_into().unwrap() {
        None
    } else {
        let next_item = next_res.pop();
        next_item.map(|item| CPagination {
            id: item.user_id,
            created_at: item.created_at.clone(),
        })
    };

    return Ok(Json(UserListResponse {
        data: next_res,
        next_cursor,
    }));
}
