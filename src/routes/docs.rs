use crate::app::ApiContext;
use crate::routes::oauth::OAuthApi;
use axum::{routing::get, Json, Router};
use utoipa::OpenApi;

pub fn router() -> Router<ApiContext> {
    Router::new().route("/api-docs/openapi.json", get(openapi))
}

#[derive(OpenApi)]
#[openapi(nest(
        (
            path = "/oauth", api = OAuthApi
        )
))]
struct Api;

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(Api::openapi())
}
