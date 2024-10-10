// TODO: Need to implement this for auto validating from request
//
// use axum::extract::{FromRequest, Json, Request};
// use serde::de::DeserializeOwned;
// use validator::Validate;
//
// use super::error::AppError;
//
// #[derive(Debug, Clone, Copy, Default)]
// pub struct ValidatedJson<T>(pub T);
//
// impl<T, S> FromRequest<S> for ValidatedJson<T>
// where
//     T: DeserializeOwned + Validate,
//     S: Send + Sync,
//     Json<T>: FromRequest<S, Rejection = AppError>,
// {
//     type Rejection = AppError;
//
//     async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
//         let Json(value) = Json::<T>::from_request(req, state).await?;
//         value.validate()?;
//         Ok(ValidatedJson(value))
//     }
// }
