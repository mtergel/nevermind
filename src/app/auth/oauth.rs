use std::collections::HashMap;

use anyhow::Context;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

pub struct OAuthClient {
    client_id: String,
    client_secret: SecretString,
    token_url: String,
    redirect_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct OAuthAccessToken {
    access_token: String,
}

impl OAuthClient {
    pub fn new(
        client_id: &str,
        client_secret: &SecretString,
        token_url: &str,
        redirect_uri: &str,
    ) -> Self {
        Self {
            client_id: client_id.to_owned(),
            client_secret: client_secret.clone(),
            token_url: token_url.to_owned(),
            redirect_uri: redirect_uri.to_owned(),
        }
    }

    /// # Security Warning
    ///
    /// Leaking this value may compromise the security of the OAuth2 flow.
    pub async fn exchange_code_for_access_token(
        &self,
        code: &str,
        client: &reqwest::Client,
    ) -> anyhow::Result<String> {
        let mut body = HashMap::new();

        // Common
        body.insert("code", code);
        body.insert("redirect_uri", &self.redirect_uri);

        // Github
        body.insert("client_id", &self.client_id);
        body.insert("client_secret", self.client_secret.expose_secret());

        // Discord
        body.insert("grant_type", "authorization_code");

        let req = client
            .post(&self.token_url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&body)
            // Discord
            .basic_auth(&self.client_id, Some(self.client_secret.expose_secret()));

        tracing::debug!("Exchange request: {:?}", req);
        let res: OAuthAccessToken = req
            .send()
            .await
            .context("failed to exchange code for token")?
            .json::<OAuthAccessToken>()
            .await
            .context("failed to deserialize as JSON")?;

        Ok(res.access_token)
    }
}
