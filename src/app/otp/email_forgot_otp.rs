use crate::app::email::client::EmailClient;

use super::OtpManager;
use anyhow::Context;
use base32::encode;
use rand::RngCore;
use redis::{AsyncCommands, Client};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const EMAIL_FORGOT_OTP_LENGTH: time::Duration = time::Duration::hours(1);

pub struct EmailForgotOtp {
    pub should_hash: bool,
}

impl EmailForgotOtp {
    fn get_db_key(&self, token: &str) -> String {
        format!("reset:{}", token)
    }

    fn get_hashed_key(&self, token: &str) -> String {
        if !self.should_hash {
            return token.to_string();
        }

        let mut hasher = Sha256::new();
        hasher.update(token);
        let hashed_token = hasher.finalize();

        hex::encode(hashed_token)
    }

    #[tracing::instrument(name = "Storing verify OTP", skip_all)]
    pub async fn store_data(
        &self,
        token: &str,
        client: &Client,
        email: &str,
    ) -> anyhow::Result<()> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let hashed_token = self.get_hashed_key(&token);

        let _: () = conn
            .set_ex(
                self.get_db_key(&hashed_token),
                email,
                EMAIL_FORGOT_OTP_LENGTH.whole_seconds() as u64,
            )
            .await
            .context("failed to store value to redis")?;

        Ok(())
    }

    #[tracing::instrument(name = "Consume forgot OTP", skip_all, fields(token = ?token))]
    pub async fn get_data(&self, token: &str, client: &Client) -> anyhow::Result<Option<String>> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let hashed_token = self.get_hashed_key(&token);

        let res: Option<String> = conn
            .get(self.get_db_key(&hashed_token))
            .await
            .context("failed to get value from redis")?;

        let _: () = conn
            .del(self.get_db_key(&hashed_token))
            .await
            .context("failed to delete key")?;

        Ok(res)
    }

    #[tracing::instrument(name = "Sending reset password instruction email", skip_all, fields(email = ?email))]
    pub async fn send_email(client: &EmailClient, token: &str, email: &str) -> anyhow::Result<()> {
        let email_content = client.build_email_confirmation(token).await?;
        client.send_email(email, email_content).await?;

        Ok(())
    }
}

impl OtpManager for EmailForgotOtp {
    #[tracing::instrument(name = "Generating reset password OTP", skip_all)]
    fn generate_otp(&self) -> String {
        // Generate 15 random bytes (15 bytes * 8 bits = 120 bits of entropy)
        let mut bytes = [0u8; 15];
        rand::thread_rng().fill_bytes(&mut bytes);

        // Encode the random bytes in Base32
        encode(base32::Alphabet::Rfc4648 { padding: true }, &bytes)
    }
}
