use super::OtpManager;
use anyhow::Context;
use rand::Rng;
use redis::{AsyncCommands, Client};
use uuid::Uuid;

pub const EMAIL_VERIFY_OTP_LENGTH: time::Duration = time::Duration::days(1);

pub struct EmailVerifyOtp {
    pub user_id: Uuid,
}

impl EmailVerifyOtp {
    fn get_email_verify_key(&self, token: &str) -> String {
        format!("user:{}:email:{}", self.user_id, token)
    }

    #[tracing::instrument(name = "Storing email into redis using OTP", skip_all)]
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

        let _: () = conn
            .set_ex(
                self.get_email_verify_key(token),
                email,
                EMAIL_VERIFY_OTP_LENGTH.whole_seconds() as u64,
            )
            .await
            .context("failed to store value to redis")?;

        Ok(())
    }

    #[tracing::instrument(name = "Getting email from redis using OTP", skip_all)]
    pub async fn get_data(&self, token: &str, client: &Client) -> anyhow::Result<Option<String>> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let res: Option<String> = conn
            .get(self.get_email_verify_key(token))
            .await
            .context("faield to get value from redis")?;

        Ok(res)
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
