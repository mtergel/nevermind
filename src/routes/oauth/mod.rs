use utoipa::OpenApi;
mod token;

#[derive(OpenApi)]
#[openapi(paths())]
pub struct OAuthApi;

pub use token::*;
