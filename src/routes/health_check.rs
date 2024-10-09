use axum::{routing::get, Router};

use crate::app::ApiContext;

pub fn router() -> Router<ApiContext> {
    Router::new().route("/health_check", get(health_check))
}

async fn health_check() {}
