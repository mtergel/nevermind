use utoipa::OpenApi;
mod token;

#[derive(OpenApi)]
#[openapi(paths(token::oauth_token))]
pub struct OAuthApi;

pub use token::*;
