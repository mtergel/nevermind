use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::{
        error::AppError,
        oauth::{
            get_or_create_user, update_missing_user_metadata, upsert_email, upsert_social_login,
            OAuthClient, UpdateUserMetadata,
        },
        utils::avatar_generator::generate_avatar,
    },
    config::AppConfig,
    routes::oauth::AssertionProvider,
};

// The user data we'll get back from Discord.
// https://discord.com/developers/docs/resources/user#user-object
#[derive(Debug, Deserialize)]
struct DiscordUser {
    pub id: String,
    pub email: Option<String>,
    pub verified: Option<bool>,
    pub avatar: Option<String>, // avatar_hash
}

/// Converts a Discord OAuth token to an access token and updates the database.
///
/// # Overview
/// `handle_discord_assertion` is responsible for handling Discord login assertions. It takes in
/// an OAuth code, uses it to request an access token from Discord, and then updates the relevant
/// database tables with the user information obtained from the access token.
///
/// # Returns
///
/// - `Result<Uuid, AppError>`: Returns a `Result` that, on success, contains the `user_id`
///   associated with the Discord user. If the operation fails at any step (e.g., invalid code,
///   network error, database error), it returns an `AppError`.
pub async fn handle_discord_assertion(
    pool: &PgPool,
    config: &AppConfig,
    http_client: &reqwest::Client,
    code: &str,
) -> Result<Uuid, AppError> {
    let discord_client = OAuthClient::new(
        &config.app_discord_id,
        &config.app_discord_secret,
        &config.app_discord_token_url,
        &format!("{}/auth/oauth", &config.app_frontend_url),
    );

    let token = discord_client
        .exchange_code_for_access_token(code, http_client)
        .await
        .context("failed to exchange code for token")?;

    let user_data: DiscordUser = http_client
        .get(format!("{}/users/@me", config.app_discord_api_base_uri))
        .header("Accept", "application/json")
        .header("User-Agent", "Let's Yahu".to_owned())
        .bearer_auth(&token)
        .send()
        .await
        .context("failed to get user details")?
        .json::<DiscordUser>()
        .await
        .context("failed to deserialize as JSON")?;

    tracing::debug!("User data: {:?}", &user_data);

    match user_data.email {
        Some(provider_email) => {
            let mut tx = pool.begin().await?;

            // Upsert db
            let user_id = get_or_create_user(&provider_email, &mut tx).await?;
            let email_id = upsert_email(
                &provider_email,
                &user_id,
                Some(true) == user_data.verified,
                &mut tx,
            )
            .await?;
            upsert_social_login(
                &email_id,
                &user_id,
                AssertionProvider::Discord,
                &user_data.id.to_string(),
                &mut tx,
            )
            .await?;

            // https://discord.com/developers/docs/reference#image-formatting
            let image = match user_data.avatar {
                Some(hash) => format!(
                    "https://cdn.discordapp.com/avatars/{}/{}",
                    user_data.id, hash
                ),
                None => generate_avatar(&user_id.to_string()),
            };

            update_missing_user_metadata(
                UpdateUserMetadata {
                    user_id,
                    bio: None,
                    image,
                },
                &mut tx,
            )
            .await?;

            tx.commit().await?;

            Ok(user_id)
        }

        None => Err(AppError::unprocessable_entity([("email", "missing")])),
    }
}
