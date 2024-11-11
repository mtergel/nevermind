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

// The user data we'll get back from Github.
// https://docs.github.com/en/rest/users/users?apiVersion=2022-11-28#get-the-authenticated-user
#[derive(Debug, Deserialize)]
struct GithubUser {
    pub id: i64,
    pub email: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

// The email data we'll get back from Github.
#[derive(Debug, Deserialize)]
struct GithubUserEmail {
    pub email: String,
    pub verified: bool,
    pub primary: bool,
}

/// Converts a GitHub OAuth token to an access token and updates the database.
///
/// # Overview
/// `handle_github_assertion` is responsible for handling GitHub login assertions. It takes in
/// an OAuth code, uses it to request an access token from GitHub, and then updates the relevant
/// database tables with the user information obtained from the access token.
///
/// # Returns
///
/// - `Result<Uuid, AppError>`: Returns a `Result` that, on success, contains the `user_id`
///   associated with the GitHub user. If the operation fails at any step (e.g., invalid code,
///   network error, database error), it returns an `AppError`.
pub async fn handle_github_assertion(
    pool: &PgPool,
    config: &AppConfig,
    http_client: &reqwest::Client,
    code: &str,
) -> Result<Uuid, AppError> {
    let git_client = OAuthClient::new(
        &config.app_github_id,
        &config.app_github_secret,
        &config.app_github_token_url,
        &format!("{}/auth/oauth", &config.app_frontend_url),
    );

    let token = git_client
        .exchange_code_for_access_token(code, http_client)
        .await
        .context("failed to exchange code for token")?;

    let user_data: GithubUser = http_client
        .get(format!("{}/user", config.app_github_api_base_uri))
        .bearer_auth(&token)
        .header("Accept", "application/json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "Let's Yahu".to_owned())
        .send()
        .await
        .context("failed to get user details")?
        .json::<GithubUser>()
        .await
        .context("failed to deserialize as JSON")?;

    tracing::debug!("User data: {:?}", &user_data);

    let mut user_data_email = user_data.email.clone();
    let mut user_email_verified = true;

    if user_data_email.is_none() {
        // fetch email
        let user_email_list: Vec<GithubUserEmail> = http_client
            .get(format!("{}/user/emails", config.app_github_api_base_uri))
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "Let's Yahu".to_owned())
            .send()
            .await
            .context("failed to get email list")?
            .json::<Vec<GithubUserEmail>>()
            .await
            .context("failed to deserialize as JSON")?;

        if user_email_list.is_empty() {
            tracing::warn!("User email list is empty {:?}", &user_data);
            user_email_verified = false;
        } else {
            // find primary email
            match user_email_list.iter().find(|x| x.primary) {
                Some(email) => {
                    user_data_email = Some(email.email.clone());
                    user_email_verified = email.verified;
                }

                None => {
                    if let Some(email) = user_email_list.first() {
                        user_data_email = Some(email.email.clone())
                    }
                }
            }
        }
    }

    match user_data_email {
        Some(provider_email) => {
            let mut tx = pool.begin().await?;

            // Upsert db
            let user_id = get_or_create_user(&provider_email, &mut tx).await?;
            let email_id =
                upsert_email(&provider_email, &user_id, user_email_verified, &mut tx).await?;
            upsert_social_login(
                &email_id,
                &user_id,
                AssertionProvider::Github,
                &user_data.id.to_string(),
                &mut tx,
            )
            .await?;

            update_missing_user_metadata(
                UpdateUserMetadata {
                    user_id,
                    bio: user_data.bio,
                    image: user_data
                        .avatar_url
                        .unwrap_or(generate_avatar(&user_id.to_string())),
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
