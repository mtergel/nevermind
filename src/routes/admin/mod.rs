use axum::{routing::get, Router};
use business::get_business;
use users::list_users;
use utoipa::OpenApi;

use crate::{app::ApiContext, permission_required};

pub mod business;
pub mod users;

fn users_router() -> Router<ApiContext> {
    Router::new()
        .route("/users", get(list_users))
        .route_layer(permission_required!(&AppPermission::UserRead))
}

fn business_router() -> Router<ApiContext> {
    Router::new()
        .route("/business/{id}", get(get_business))
        // TODO: Permission setup
        .route_layer(permission_required!(&AppPermission::UserRead))
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
