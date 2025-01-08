use axum::{routing::get, Router};
use users::list_users;
use utoipa::OpenApi;

use crate::{app::ApiContext, permission_required};

pub mod users;
pub mod business;

fn users_router() -> Router<ApiContext> {
    Router::new()
        .route("/users", get(list_users))
        .route_layer(permission_required!(&AppPermission::UserView))
}

fn business_router() -> Router<ApiContext> {
    Router::new()
        .route("/business", get(list_users))
        // TODO: Permission setup
        .route_layer(permission_required!(&AppPermission::UserView))
}

pub fn router() -> Router<ApiContext> {
    Router::new().nest(
        "/admin",
        Router::new().merge(users_router()).merge(business_router()),
    )
}

#[derive(OpenApi)]
#[openapi(paths(users::list_users))]
pub struct AdminApi;
