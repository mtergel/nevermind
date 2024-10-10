use axum::{extract::State, routing::post, Json, Router};
use secrecy::SecretString;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

use crate::app::{error::AppError, ApiContext};

#[derive(Debug, Deserialize, Validate, ToSchema)]
struct LoginUser {
    #[validate(email)]
    email: String,

    #[schema(value_type = String)]
    password: SecretString,
}

pub fn router() -> Router<ApiContext> {
    Router::new().route("/auth/login", post(login_user))
}

#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginUser,
    responses(
        (status = 200, description = "Successful login"),
        (status = 400, description = "Bad request", body = AppError),
        (status = 422, description = "Invalid input", body = AppError),
    )
)]
#[tracing::instrument(name = "Login user", skip_all, fields(email = %req.email))]
async fn login_user(ctx: State<ApiContext>, Json(req): Json<LoginUser>) -> Result<(), AppError> {
    // Doing this manually untill its fixed in extrator.rs
    if let Err(e) = req.validate() {
        tracing::info!("Invalid login input");
        return Err(e.into());
    }

    Ok(())
}
