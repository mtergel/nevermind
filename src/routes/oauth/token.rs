use anyhow::Context;
use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, TokenResponse, TokenUrl,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    app::{
        auth::{
            password::{validate_credentials, Credentials},
            scope::get_scopes,
            session::{Session, SessionMetadata},
            token::{RefreshTokenClaims, TokenManager, ValidateTokenError},
        },
        error::AppError,
        extrator::ValidatedJson,
        ApiContext,
    },
    config::AppConfig,
    routes::docs::AUTH_TAG,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
struct GrantTokenInput {
    grant_type: GrantType,

    // Refresh token grant inputs
    refresh_token: Option<String>,

    // Password grant inputs
    #[validate(email)]
    email: Option<String>,
    #[schema(value_type = Option<String>)]
    password: Option<SecretString>,

    // Assertion grant inputs
    code: Option<String>,
    provider: Option<AssertionProvider>,
}

#[derive(Debug, Serialize, ToSchema)]
struct GrantResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    token_type: TokenType,
    scope: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
enum GrantType {
    #[serde(rename = "password")]
    Password,
    #[serde(rename = "refresh_token")]
    RefreshToken,
    #[serde(rename = "assertion")]
    Assertion,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
enum TokenType {
    #[serde(rename = "bearer")]
    Bearer,
}

pub fn router() -> Router<ApiContext> {
    Router::new().route("/oauth/token", post(oauth_token))
}

#[utoipa::path(
    post,
    path = "/token",
    tag = AUTH_TAG,
    request_body = GrantTokenInput,
    responses(
        (status = 200, description = "Successful grant", body = GrantResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Refresh token expired"),
        (status = 422, description = "Invalid input", body = AppError),
        (status = 500, description = "Internal server error")
    )
)]
#[tracing::instrument(name = "Grant token", skip_all, fields(grant = tracing::field::Empty, user_id = tracing::field::Empty))]
async fn oauth_token(
    ctx: State<ApiContext>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<GrantTokenInput>,
) -> Result<Json<GrantResponse>, AppError> {
    tracing::Span::current().record("grant", tracing::field::display(&req.grant_type));

    let metadata = SessionMetadata {
        device_name: headers
            .get("X-User-Agent")
            .and_then(|hv| hv.to_str().ok())
            .map(|s| s.to_string()),
        ip: headers
            .get("X-Forwarded-For")
            .and_then(|hv| hv.to_str().ok())
            .map(|s| s.to_string()),
        last_accessed: OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap(),
    };

    match req.grant_type {
        GrantType::Password => {
            let owner_input = OwnerPasswordFlowInput::try_from(req)?;
            let res = owner_password_flow(
                owner_input,
                metadata,
                &ctx.db_pool,
                &ctx.redis_client,
                &ctx.token_manager,
            )
            .await?;

            Ok(Json(res))
        }
        GrantType::RefreshToken => {
            let rotate_input = RefreshTokenInput::try_from(req)?;
            let res = refresh_token_flow(
                rotate_input,
                metadata,
                &ctx.db_pool,
                &ctx.redis_client,
                &ctx.token_manager,
            )
            .await?;

            Ok(Json(res))
        }
        GrantType::Assertion => {
            let assertion_input = AssertionFlowInput::try_from(req)?;
            let res = assertion_flow(
                assertion_input,
                metadata,
                &ctx.db_pool,
                &ctx.redis_client,
                &ctx.token_manager,
                &ctx.config,
            )
            .await?;

            Ok(Json(res))
        }
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
struct OwnerPasswordFlowInput {
    #[validate(email)]
    email: String,

    #[schema(value_type = String)]
    password: SecretString,
}

#[tracing::instrument(name = "Owner password flow", skip_all)]
async fn owner_password_flow(
    req: OwnerPasswordFlowInput,
    metadata: SessionMetadata,
    pool: &PgPool,
    client: &redis::Client,
    token_manager: &TokenManager,
) -> Result<GrantResponse, AppError> {
    let credentials = Credentials {
        email: req.email,
        password_hash: req.password,
    };

    match validate_credentials(credentials, pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));

            let session = Session::new(user_id);
            let tokens = session.insert(metadata, client, token_manager).await?;

            let scopes = get_scopes(user_id, pool).await?;

            Ok(GrantResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_in: tokens.expires_in,
                token_type: TokenType::Bearer,
                scope: scopes.to_string(),
            })
        }

        Err(e) => Err(e),
    }
}

#[derive(Debug, Deserialize, ToSchema)]
struct RefreshTokenInput {
    refresh_token: String,
}

#[tracing::instrument(name = "Refresh token flow", skip_all)]
async fn refresh_token_flow(
    req: RefreshTokenInput,
    metadata: SessionMetadata,
    pool: &PgPool,
    client: &redis::Client,
    token_manager: &TokenManager,
) -> Result<GrantResponse, AppError> {
    let claims: RefreshTokenClaims =
        token_manager
            .verify(&req.refresh_token)
            .await
            .map_err(|e| match e {
                ValidateTokenError::ParseError => {
                    AppError::unprocessable_entity([("refresh_token", "parse")])
                }
                _ => AppError::Unauthorized,
            })?;

    let session = Session {
        user_id: claims.sub,
        session_id: claims.sid,
    };

    // check if session is still in storage
    let _session_data = session.get_data(client).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&claims.sub));

    // if session is valid
    let tokens = session.renew(metadata, client, token_manager).await?;
    let scopes = get_scopes(claims.sub, pool).await?;

    Ok(GrantResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        token_type: TokenType::Bearer,
        scope: scopes.to_string(),
    })
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
struct AssertionFlowInput {
    code: String,
    provider: AssertionProvider,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "social_provider", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
enum AssertionProvider {
    Github,
    #[serde(skip)]
    Google,
    #[serde(skip)]
    Facebook,
    #[serde(skip)]
    Discord,
}

#[tracing::instrument(name = "Assertion flow", skip_all)]
async fn assertion_flow(
    req: AssertionFlowInput,
    metadata: SessionMetadata,
    pool: &PgPool,
    client: &redis::Client,
    token_manager: &TokenManager,
    config: &AppConfig,
) -> Result<GrantResponse, AppError> {
    // For each provider
    // Handle getting access_token
    // Then convert access_token to User
    // Signup new user or merge with existing user
    // Then return session tokens

    match req.provider {
        AssertionProvider::Github => {
            let git_id = ClientId::new(config.app_github_id.clone());
            let git_secret =
                ClientSecret::new(config.app_github_secret.expose_secret().to_string());
            let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
                .context("github invalid authorization endpoint")
                .unwrap();
            let token_url =
                TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
                    .context("github invalid token endpoint")
                    .unwrap();

            // The user data we'll get back from Github.
            // https://docs.github.com/en/rest/users/users?apiVersion=2022-11-28#get-the-authenticated-user
            #[derive(Deserialize)]
            struct GithubUser {
                pub id: i64,
                pub email: Option<String>,
                pub bio: Option<String>,
                pub avatar_url: Option<String>,
            }

            // TODO:
            // This should be created on config and passed
            let git_client = BasicClient::new(git_id, Some(git_secret), auth_url, Some(token_url));
            let token = git_client
                .exchange_code(AuthorizationCode::new(req.code))
                .request_async(async_http_client)
                .await
                .context("failed to exchange code for token")?;

            let http_client = reqwest::Client::new();
            let user_data: GithubUser = http_client
                .get("https://api.github.com/user")
                .bearer_auth(token.access_token().secret())
                .send()
                .await
                .context("failed to get user details")?
                .json::<GithubUser>()
                .await
                .context("failed to deserialize as JSON")?;

            match user_data.email {
                Some(provider_email) => {
                    let mut tx = pool.begin().await?;

                    // Upsert db
                    let user_id = get_or_create_user(&provider_email, &mut tx).await?;
                    let email_id = upsert_email(&provider_email, &user_id, &mut tx).await?;
                    upsert_social_login(
                        &email_id,
                        AssertionProvider::Github,
                        &user_data.id.to_string(),
                        &mut tx,
                    )
                    .await?;

                    update_missing_user_metadata(
                        UpdateUserMetadata {
                            user_id,
                            bio: user_data.bio,
                            image: user_data.avatar_url,
                        },
                        &mut tx,
                    )
                    .await?;

                    tx.commit().await?;

                    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

                    let session = Session::new(user_id);
                    let tokens = session.insert(metadata, client, token_manager).await?;
                    let scopes = get_scopes(user_id, pool).await?;

                    Ok(GrantResponse {
                        access_token: tokens.access_token,
                        refresh_token: tokens.refresh_token,
                        expires_in: tokens.expires_in,
                        token_type: TokenType::Bearer,
                        scope: scopes.to_string(),
                    })
                }

                None => return Err(AppError::unprocessable_entity([("email", "missing")])),
            }
        }
        AssertionProvider::Discord => Err(AppError::NotFound),
        AssertionProvider::Google => Err(AppError::NotFound),
        AssertionProvider::Facebook => Err(AppError::NotFound),
    }
}

impl std::fmt::Display for GrantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrantType::Password => write!(f, "password"),
            GrantType::RefreshToken => write!(f, "refresh_token"),
            GrantType::Assertion => write!(f, "assertion"),
        }
    }
}

impl TryFrom<GrantTokenInput> for OwnerPasswordFlowInput {
    type Error = AppError;
    fn try_from(value: GrantTokenInput) -> Result<Self, Self::Error> {
        let email = value
            .email
            .ok_or(AppError::unprocessable_entity([("email", "missing")]))?;

        let password = value
            .password
            .ok_or(AppError::unprocessable_entity([("password", "missing")]))?;

        let input = OwnerPasswordFlowInput { email, password };
        input.validate()?;

        Ok(input)
    }
}

impl TryFrom<GrantTokenInput> for RefreshTokenInput {
    type Error = AppError;
    fn try_from(value: GrantTokenInput) -> Result<Self, Self::Error> {
        let refresh_token = value.refresh_token.ok_or(AppError::unprocessable_entity([(
            "refresh_token",
            "missing",
        )]))?;

        let input = RefreshTokenInput { refresh_token };

        Ok(input)
    }
}

impl TryFrom<GrantTokenInput> for AssertionFlowInput {
    type Error = AppError;
    fn try_from(value: GrantTokenInput) -> Result<Self, Self::Error> {
        let code = value
            .code
            .ok_or(AppError::unprocessable_entity([("code", "missing")]))?;

        let provider = value
            .provider
            .ok_or(AppError::unprocessable_entity([("provider", "missing")]))?;

        let input = AssertionFlowInput { code, provider };

        Ok(input)
    }
}

/// Gets previous or newly created user's UUID
async fn get_or_create_user(
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
                    insert into "user" (username, password_hash, reset_password)
                    values ($1, $2, true)
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

async fn upsert_email(
    email: &str,
    user_id: &Uuid,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<Uuid> {
    let email_id = sqlx::query_scalar!(
        r#"
            insert into email (email, user_id)
            values ($1, $2)

            on conflict (email)
            do update set 
            verified = true,
            confirmation_sent_at = null

            returning email_id
        "#,
        email,
        user_id
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(email_id)
}

async fn upsert_social_login(
    email_id: &Uuid,
    provider: AssertionProvider,
    provider_user_id: &str,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
            insert into social_login (email_id, provider, provider_user_id)
            values ($1, $2, $3)
        "#,
        email_id,
        provider as _,
        provider_user_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[derive(serde::Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
struct UpdateUserMetadata {
    user_id: Uuid,
    bio: Option<String>,
    image: Option<String>,
}

async fn update_missing_user_metadata(
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
