use std::collections::HashSet;

use axum::{
    extract::{FromRequest, FromRequestParts, Json, Request},
    http::{header::ACCEPT_LANGUAGE, request::Parts},
};
use serde::de::DeserializeOwned;
use uuid::Uuid;
use validator::Validate;

use super::{auth::scope::AppPermission, error::AppError, utils::types::Locale};

/// Add this as a parameter to a handler function to
/// extract body into validated JSON.
/// The request will be rejected if it doesn't pass axum::Json's requirements
/// plus the validation rules. It will return 422 for validation rules.
///
/// ⚠️ Since parsing JSON requires consuming the request body, the `Json` extractor must be
/// *last* if there are multiple extractors in a handler.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

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

pub struct ExtractLocale(pub Locale);
impl<S> FromRequestParts<S> for ExtractLocale
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(locale) = parts.headers.get(ACCEPT_LANGUAGE) {
            let locale = locale.to_str().unwrap_or("en").parse().unwrap();
            return Ok(ExtractLocale(locale));
        }

        Ok(ExtractLocale(Locale::En))
    }
}
