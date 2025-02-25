use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
        oauth::{discord::handle_discord_assertion, github::handle_github_assertion},
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
        (status = 403, description = "Reset password required"),
        (status = 404, description = "Unimplemented or inactive provider"),
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
                &ctx.http_client,
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

            let scopes = get_scopes(user_id, pool).await?;
            let session = Session::new(user_id);
            let tokens = session
                .insert(metadata, client, token_manager, &scopes.to_string())
                .await?;

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
    let scopes = get_scopes(claims.sub, pool).await?;
    let tokens = session
        .renew(metadata, client, token_manager, &scopes.to_string())
        .await?;

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
pub enum AssertionProvider {
    Github,
    Discord,
    #[serde(skip)]
    Google,
    #[serde(skip)]
    Facebook,
}

#[tracing::instrument(name = "Assertion flow", skip_all, fields(req = ?req))]
async fn assertion_flow(
    req: AssertionFlowInput,
    metadata: SessionMetadata,
    pool: &PgPool,
    client: &redis::Client,
    token_manager: &TokenManager,
    config: &AppConfig,
    http_client: &reqwest::Client,
) -> Result<GrantResponse, AppError> {
    let user_id: Uuid = match req.provider {
        AssertionProvider::Github => {
            handle_github_assertion(pool, config, http_client, &req.code).await?
        }
        AssertionProvider::Discord => {
            handle_discord_assertion(pool, config, http_client, &req.code).await?
        }
        AssertionProvider::Google => return Err(AppError::NotFound),
        AssertionProvider::Facebook => return Err(AppError::NotFound),
    };

    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    let session = Session::new(user_id);
    let scopes = get_scopes(user_id, pool).await?;
    let tokens = session
        .insert(metadata, client, token_manager, &scopes.to_string())
        .await?;

    Ok(GrantResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        token_type: TokenType::Bearer,
        scope: scopes.to_string(),
    })
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
