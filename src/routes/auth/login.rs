use axum::{extract::State, routing::post, Router};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::app::{
    error::AppError,
    extrator::ValidatedJson,
    password::{validate_credentials, Credentials},
    ApiContext,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
struct LoginUser {
    #[validate(email)]
    email: String,

    #[schema(value_type = String)]
    password: SecretString,
}

#[derive(Debug, Serialize, ToSchema)]
struct UserResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    token_type: TokenType,
    scope: Scope,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
enum TokenType {
    Bearer,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
enum Scope {}

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
#[tracing::instrument(name = "Login user", skip_all, fields(email = tracing::field::Empty, user_id = tracing::field::Empty))]
async fn login_user(
    ctx: State<ApiContext>,
    ValidatedJson(req): ValidatedJson<LoginUser>,
) -> Result<(), AppError> {
    tracing::Span::current().record("email", tracing::field::display(&req.email));

    let credentials = Credentials {
        email: req.email,
        password_hash: req.password,
    };

    match validate_credentials(credentials, &ctx.db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));

            Ok(())
        }

        Err(e) => Err(e),
    }
}
