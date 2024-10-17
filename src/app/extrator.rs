use axum::{
    async_trait,
    extract::{FromRef, FromRequest, FromRequestParts, Json, Request},
    http::{header::AUTHORIZATION, request::Parts},
};
use serde::de::DeserializeOwned;
use uuid::Uuid;
use validator::Validate;

use super::{auth::token::AccessTokenClaims, error::AppError, ApiContext};

/// Add this as a parameter to a handler function to
/// extract body into validated JSON.
/// The request will be rejected if it doesn't pass axum::Json's requirements
/// plus the validation rules. It will return 422 for validation rules.
///
/// ⚠️ Since parsing JSON requires consuming the request body, the `Json` extractor must be
/// *last* if there are multiple extractors in a handler.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}

/// Add this as a parameter to a handler function to require the user to be logged in.
///
/// Parses a JWT from the `Authorization: Bearer <token>` header.
pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

const SCHEME_PREFIX: &str = "Bearer ";

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    ApiContext: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx: ApiContext = ApiContext::from_ref(state);

        // Get the value of the `Authorization` header, if it was sent at all.
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or(AppError::Unauthorized)?;

        let auth_header = auth_header.to_str().map_err(|_| AppError::Unauthorized)?;
        if !auth_header.starts_with(SCHEME_PREFIX) {
            return Err(AppError::Unauthorized);
        }

        let token = &auth_header[SCHEME_PREFIX.len()..];
        let user = ctx.token_manager.verify::<AccessTokenClaims>(token).await?;

        Ok(AuthUser {
            user_id: user.sub,
            session_id: user.sid,
        })
    }
}
