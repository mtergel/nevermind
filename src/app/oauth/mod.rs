use std::collections::HashMap;

use anyhow::Context;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::routes::oauth::AssertionProvider;

pub mod discord;
pub mod github;

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

// Helper methods

/// Gets previous or newly created user's UUID
pub async fn get_or_create_user(
    email: &str,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<Uuid> {
    let user_id = sqlx::query_scalar!(
        r#"
            select user_id
            from email
            where email = $1
        "#,
        email
    )
    .fetch_optional(&mut **tx)
    .await?;

    match user_id {
        Some(user_id) => Ok(user_id),

        None => {
            let new_username = Uuid::new_v4();
            let new_password = Uuid::new_v4();

            let user_id = sqlx::query_scalar!(
                r#"
                    insert into "user" (username, password_hash, reset_password, reset_username)
                    values ($1, $2, true, true)
                    returning user_id
                "#,
                new_username.to_string(),
                new_password.to_string()
            )
            .fetch_one(&mut **tx)
            .await?;

            Ok(user_id)
        }
    }
}

pub async fn upsert_email(
    email: &str,
    user_id: &Uuid,
    verified: bool,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<Uuid> {
    let primary_email = sqlx::query_scalar!(
        r#"
            select email_id
            from email
            where user_id = $1 and is_primary = true
            limit 1
        "#,
        user_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    let email_id = sqlx::query_scalar!(
        r#"
            insert into email (email, user_id, verified, is_primary)
            values ($1, $2, $3, $4)

            on conflict (email)
            do update set 
            verified = true,
            confirmation_sent_at = null

            returning email_id
        "#,
        email,
        user_id,
        verified,
        primary_email.is_none()
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(email_id)
}

pub async fn upsert_social_login(
    email_id: &Uuid,
    user_id: &Uuid,
    provider: AssertionProvider,
    provider_user_id: &str,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
            insert into social_login (email_id, user_id, provider, provider_user_id)
            values ($1, $2, $3, $4)
            on conflict (provider_user_id) do nothing
        "#,
        email_id,
        user_id,
        provider as _,
        provider_user_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[derive(serde::Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct UpdateUserMetadata {
    user_id: Uuid,
    bio: Option<String>,
    image: String,
}

pub async fn update_missing_user_metadata(
    input: UpdateUserMetadata,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
            update "user"
                set bio = coalesce("user".bio, $1),
                image = coalesce("user".image, $2) 
            where user_id = $3
        "#,
        input.bio,
        input.image,
        input.user_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
