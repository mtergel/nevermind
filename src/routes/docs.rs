use crate::app::ApiContext;
use crate::routes::{auth::AuthApi, oauth::OAuthApi, upload::UploadApi};
use axum::{routing::get, Json, Router};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::Components;
use utoipa::{Modify, OpenApi};

pub fn router() -> Router<ApiContext> {
    Router::new().route("/api-docs/openapi.json", get(openapi))
}

pub const AUTH_TAG: &str = "auth";
pub const EMAIL_TAG: &str = "email";
pub const UPLOAD_TAG: &str = "upload";

#[derive(OpenApi)]
#[openapi(
    nest(
        (
            path = "/oauth", api = OAuthApi
        ),
        (
            path = "/auth", api = AuthApi
        ),
        (
            path = "/upload", api = UploadApi
        )
    ),
    modifiers(&SecurityAddon)
)]
struct Api;

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(Api::openapi())
}

// OAuth Authorization header
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if openapi.components.is_none() {
            openapi.components = Some(Components::new());
        }

        openapi.components.as_mut().unwrap().add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}
