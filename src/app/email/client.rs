use aws_config::SdkConfig;
use aws_sdk_sesv2::{
    types::{Destination, EmailContent, Template},
    Client,
};
use serde::Serialize;

#[derive(Clone)]
pub struct EmailClient {
    ses_client: Client,
    verified_email: String,
    frontend_url: String,

    /// Temp solution
    /// Should probably change later
    should_mock: bool,
}

impl EmailClient {
    /// Build an email client
    ///
    /// It should only be called once, and shared
    pub fn new(
        sdk_config: &SdkConfig,
        verified_email: String,
        frontend_url: String,
        should_mock: bool,
    ) -> Self {
        let ses_client = Client::new(sdk_config);

        EmailClient {
            ses_client,
            verified_email,
            frontend_url,
            should_mock,
        }
    }

    #[tracing::instrument(name = "Building confirmation email content", skip_all)]
    pub async fn build_email_confirmation(&self, token: &str) -> anyhow::Result<EmailContent> {
        let confirmation_url = format!("{}/account/verify?token={}", self.frontend_url, token);

        #[derive(Serialize)]
        struct EmailVerifyData {
            verification_link: String,
            code: String,
        }

        let email_data = EmailVerifyData {
            verification_link: confirmation_url,
            code: token.to_string(),
        };

        let email_content = EmailContent::builder()
            .template(
                Template::builder()
                    .template_name("")
                    .template_data(serde_json::to_string(&email_data).unwrap())
                    .build(),
            )
            .build();

        Ok(email_content)
    }

    #[tracing::instrument(name = "Sending email", skip_all, fields(email = ?email))]
    pub async fn send_email(&self, email: &str, email_content: EmailContent) -> anyhow::Result<()> {
        if self.should_mock {
            return Ok(());
        }

        match self
            .ses_client
            .send_email()
            .from_email_address(&self.verified_email)
            .destination(Destination::builder().to_addresses(email).build())
            .content(email_content)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Error sending newsletter to {}: {}",
                    email,
                    e
                ))
            }
        }
    }
}
