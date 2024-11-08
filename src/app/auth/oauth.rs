use anyhow::Context;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, TokenResponse, TokenUrl,
};
use secrecy::{ExposeSecret, SecretString};

pub struct OAuthClient {
    client: BasicClient,
}

impl OAuthClient {
    pub fn new(
        client_id: &String,
        client_secret: &SecretString,
        auth_url: &String,
        token_url: &String,
    ) -> Self {
        let c_id = ClientId::new(client_id.to_string());
        let c_s = ClientSecret::new(client_secret.expose_secret().to_string());
        let a_url = AuthUrl::new(auth_url.to_string())
            .context("invalid authorization endpoint")
            .unwrap();
        let t_url = TokenUrl::new(token_url.to_string())
            .context("invalid token endpoint")
            .unwrap();

        let client = BasicClient::new(c_id, Some(c_s), a_url, Some(t_url));

        OAuthClient { client }
    }

    /// # Security Warning
    ///
    /// Leaking this value may compromise the security of the OAuth2 flow.
    ///
    pub async fn exchange_code_for_access_token(&self, code: &str) -> anyhow::Result<String> {
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await?;

        let access_token = token.access_token().secret();

        Ok(access_token.clone())
    }
}
