use crate::app::email::client::EmailClient;

use super::OtpManager;
use anyhow::Context;
use rand::Rng;
use redis::{AsyncCommands, Client};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const EMAIL_VERIFY_OTP_LENGTH: time::Duration = time::Duration::days(1);

pub struct EmailVerifyOtp {
    pub user_id: Uuid,
    pub should_hash: bool,
}

impl EmailVerifyOtp {
    fn get_db_key(&self, token: &str) -> String {
        format!("user:{}:email:{}", self.user_id, token)
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

        let hashed_token = self.get_hashed_key(token);

        let _: () = conn
            .set_ex(
                self.get_db_key(&hashed_token),
                email,
                EMAIL_VERIFY_OTP_LENGTH.whole_seconds() as u64,
            )
            .await
            .context("failed to store value to redis")?;

        Ok(())
    }

    #[tracing::instrument(name = "Consume verify OTP", skip_all, fields(token = ?token))]
    pub async fn get_data(&self, token: &str, client: &Client) -> anyhow::Result<Option<String>> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let hashed_token = self.get_hashed_key(token);

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

    #[tracing::instrument(name = "Get verify otps", skip_all, fields(email = ?email))]
    pub async fn get_keys(&self, client: &Client, email: &str) -> anyhow::Result<Vec<String>> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let pattern = self.get_db_key("*");
        let mut iter: redis::AsyncIter<String> = conn
            .scan_match(pattern)
            .await
            .expect("failed to scan iterate to redis");

        let mut otps: Vec<String> = Vec::new();

        while let Some(otp) = iter.next_item().await {
            otps.push(otp);
        }

        drop(iter);

        Ok(otps)
    }

    #[tracing::instrument(name = "Sending confirmation email", skip_all, fields(email = ?email))]
    pub async fn send_email(client: &EmailClient, token: &str, email: &str) -> anyhow::Result<()> {
        let email_content = client
            .build_email_confirmation(token, EMAIL_VERIFY_OTP_LENGTH.whole_hours())
            .await?;
        client.send_email(email, email_content).await?;

        Ok(())
    }
}

impl OtpManager for EmailVerifyOtp {
    #[tracing::instrument(name = "Generating email verify OTP", skip_all)]
    fn generate_otp(&self) -> String {
        let characters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let code: String = (0..8)
            .map(|_| {
                let idx = rand::thread_rng().gen_range(0..characters.len());
                characters.chars().nth(idx).unwrap()
            })
            .collect();

        code
    }
}
