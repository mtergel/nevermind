use anyhow::Context;
use aws_config::SdkConfig;
use aws_sdk_s3::presigning::{PresignedRequest, PresigningConfig};
use aws_sdk_s3::Client;

const UPLOAD_EXPIRES_IN: std::time::Duration = std::time::Duration::from_secs(60 * 5);

pub struct S3Storage {
    s3_client: Client,
    bucket_name: String,
    base_url: String,
}

impl S3Storage {
    pub fn new(sdk_config: &SdkConfig, bucket_name: &str, base_url: &str) -> Self {
        let client = Client::new(sdk_config);

        Self {
            s3_client: client,
            bucket_name: bucket_name.to_string(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn generate_upload_presigned_url(
        &self,
        full_path: String,
        file_type: String,
        file_size: i64,
    ) -> anyhow::Result<PresignedRequest> {
        let presigned = self
            .s3_client
            .put_object()
            .bucket(&self.bucket_name)
            .key(full_path)
            .content_type(file_type)
            .content_length(file_size)
            .presigned(
                PresigningConfig::builder()
                    .expires_in(UPLOAD_EXPIRES_IN)
                    .build()
                    .expect("expire must be less than one week"),
            )
            .await
            .context("failed to generate upload presigned_url")
            .unwrap();

        Ok(presigned)
    }

    pub fn get_prefixed_url(&self, path: Option<String>) -> Option<String> {
        path.map(|p| format!("{}/{}", self.base_url, p))
    }
}
