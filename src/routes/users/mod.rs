use axum::{routing::get, Router};
use users::list_users;
use utoipa::OpenApi;

use crate::{app::ApiContext, permission_required};

pub mod users;

pub fn router() -> Router<ApiContext> {
    Router::new()
        .route("/users", get(list_users))
        .route_layer(permission_required!(&AppPermission::UserView))
}

#[derive(OpenApi)]
#[openapi(paths(users::list_users))]
pub struct UsersApi;
