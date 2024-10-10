use utoipa::OpenApi;
mod login;

#[derive(OpenApi)]
#[openapi(paths(login::login_user))]
pub struct AuthApi;

pub use login::*;
