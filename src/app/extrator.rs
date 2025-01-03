use std::collections::HashSet;

use axum::{
    async_trait,
    extract::{FromRef, FromRequest, FromRequestParts, Json, Request},
    http::request::Parts,
};
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use uuid::Uuid;
use validator::Validate;

use super::{auth::scope::AppPermission, error::AppError, ApiContext};

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
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub scopes: HashSet<AppPermission>,
}

impl AuthUser {
    pub fn has_permission(&self, permission: &AppPermission) -> bool {
        self.scopes.contains(permission)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or(anyhow::anyhow!("Can't extract auth user. Wrap with login_required").into())
    }
}

// TODO:
// This should not be a extractor
// There is no data to extract
// Move this to middleware
const API_KEY_HEADER: &str = "X-Api-Key";

/// Add this as a parameter to a handler function to require a api key to process.
///
/// Parses a key from the `X-Api-Key: <token>` header.
#[derive(Debug)]
pub struct ApiKey;

#[async_trait]
impl<S> FromRequestParts<S> for ApiKey
where
    S: Send + Sync,
    ApiContext: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx: ApiContext = ApiContext::from_ref(state);

        let auth_header = parts
            .headers
            .get(API_KEY_HEADER)
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header.to_str().map_err(|_| AppError::Unauthorized)?;
        if token != ctx.config.api_key.expose_secret() {
            return Err(AppError::Unauthorized);
        }

        Ok(ApiKey)
    }
}
