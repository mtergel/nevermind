use mime2::Mime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub enum S3Path {
    Profile,
}

impl std::fmt::Display for S3Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S3Path::Profile => write!(f, "profile"),
        }
    }
}

impl S3Path {
    pub fn get_max_size(&self) -> i64 {
        match self {
            S3Path::Profile => 500_000,
        }
    }

    pub fn is_allowed_type(&self, mime: &Mime) -> bool {
        match self {
            S3Path::Profile => {
                let types = [mime2::image::JPEG, mime2::image::PNG, mime2::image::WEBP];

                types.contains(mime)
            }
        }
    }
}
