use axum::{extract::State, routing::post, Json, Router};
use secrecy::SecretString;
use serde::Deserialize;
use validator::Validate;

use crate::app::{error::AppError, ApiContext};

#[derive(Debug, Deserialize, Validate)]
struct LoginUser {
    #[validate(email)]
    email: String,
    password: SecretString,
}

pub fn router() -> Router<ApiContext> {
    Router::new().route("/login", post(login_user))
}

#[tracing::instrument(name = "Login user", skip_all, fields(email = %req.email))]
async fn login_user(ctx: State<ApiContext>, Json(req): Json<LoginUser>) -> Result<(), AppError> {
    // Doing this manually untill its fixed in extrator
    if let Err(e) = req.validate() {
        tracing::info!("Invalid login input");
        return Err(e.into());
    }

    Ok(())
}
