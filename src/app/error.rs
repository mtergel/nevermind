use std::{borrow::Cow, collections::HashMap};

use axum::{
    extract::rejection::JsonRejection,
    http::{header::WWW_AUTHENTICATE, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use sqlx::error::DatabaseError;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Error, Debug, Serialize, ToSchema)]
pub enum AppError {
    #[error("authentication required")]
    #[serde(skip)]
    Unauthorized,

    #[error("authorization required")]
    #[serde(skip)]
    Forbidden,

    #[error("requested data not found")]
    #[serde(skip)]
    NotFound,

    #[error("malformed input in the request body")]
    #[serde(skip)]
    AxumJsonRejection(#[from] JsonRejection),

    #[error("error in the request body")]
    UnprocessableEntity {
        errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
    },

    #[error("request body does not meet validation requirements")]
    #[serde(skip)]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("an error occurred with the database")]
    #[serde(skip)]
    Sqlx(#[from] sqlx::Error),

    #[error("an internal server error occurred")]
    #[serde(skip)]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct InputErrorResponse {
    errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthorized => {
                return (
                    self.status_code(),
                    // Include the `WWW-Authenticate` challenge required in the specification
                    // for the `401 Unauthorized` response code:
                    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/401
                    [(WWW_AUTHENTICATE, "Bearer")],
                )
                    .into_response();
            }
            Self::ValidationError(e) => {
                let mut error_map: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>> =
                    HashMap::new();

                for (field, error) in e.field_errors() {
                    if let Some(validation_error) = error.first() {
                        error_map
                            .entry(field.into())
                            .or_default()
                            .push(validation_error.code.clone());
                    }
                }

                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(InputErrorResponse { errors: error_map }),
                )
                    .into_response();
            }

            Self::UnprocessableEntity { errors } => {
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(InputErrorResponse { errors }),
                )
                    .into_response();
            }

            Self::Sqlx(ref e) => {
                if let sqlx::Error::RowNotFound = e {
                    return (StatusCode::NOT_FOUND).into_response();
                }

                tracing::error!("Database error: {:?}", e)
            }

            Self::Anyhow(ref e) => {
                tracing::error!("internal server error: {:?}", e)
            }

            _ => (),
        }

        (self.status_code()).into_response()
    }
}

impl AppError {
    /// Convenient constructor for `Error::UnprocessableEntity`.
    ///
    /// Multiple for the same key are collected into a list for that key.
    pub fn unprocessable_entity<K, V>(errors: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        let mut error_map = HashMap::new();

        for (key, val) in errors {
            error_map
                .entry(key.into())
                .or_insert_with(Vec::new)
                .push(val.into());
        }

        Self::UnprocessableEntity { errors: error_map }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::AxumJsonRejection(_) => StatusCode::BAD_REQUEST,
            Self::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Sqlx(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// A little helper trait for more easily converting database constraint errors into API errors.
///
/// ```rust,ignore
/// let user_id = sqlx::query_scalar!(
///     r#"insert into "user" (username, email, password_hash) values ($1, $2, $3) returning user_id"#,
///     username,
///     email,
///     password_hash
/// )
///     .fetch_one(&ctx.db_pool)
///     .await
///     .on_constraint("user_username_key", |_| Error::unprocessable_entity([("username", "already taken")]))?;
/// ```
pub trait ResultExt<T> {
    /// If `self` contains a SQLx database constraint error with the given name,
    /// transform the error.
    ///
    /// Otherwise, the result is passed through unchanged.
    fn on_constraint(
        self,
        name: &str,
        f: impl FnOnce(Box<dyn DatabaseError>) -> AppError,
    ) -> Result<T, AppError>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Into<AppError>,
{
    fn on_constraint(
        self,
        name: &str,
        map_err: impl FnOnce(Box<dyn DatabaseError>) -> AppError,
    ) -> Result<T, AppError> {
        self.map_err(|e| match e.into() {
            AppError::Sqlx(sqlx::Error::Database(dbe)) if dbe.constraint() == Some(name) => {
                map_err(dbe)
            }
            e => e,
        })
    }
}
