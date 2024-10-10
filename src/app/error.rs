use std::{borrow::Cow, collections::HashMap};

use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("malformed input in the request body")]
    AxumJsonRejection(#[from] JsonRejection),

    #[error("error in the request body")]
    UnprocessableEntity {
        errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
    },

    #[error("request body does not meet requirments")]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("an internal server error occurred")]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct InputErrorResponse {
    errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::ValidationError(e) => {
                let mut error_map: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>> =
                    HashMap::new();

                for (field, error) in e.field_errors() {
                    if let Some(validation_error) = error.first() {
                        error_map
                            .entry(field.into())
                            .or_insert_with(Vec::new)
                            .push(validation_error.code.clone().into());
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

            Self::Anyhow(ref e) => {
                tracing::error!("Internal server error: {:?}", e)
            }

            _ => (),
        }

        (self.status_code(), self.to_string()).into_response()
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
            Self::AxumJsonRejection(_) => StatusCode::BAD_REQUEST,
            Self::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
