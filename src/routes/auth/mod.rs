use utoipa::OpenApi;
mod register;

#[derive(OpenApi)]
#[openapi(paths(register::register_user))]
pub struct AuthApi;

pub use register::*;
