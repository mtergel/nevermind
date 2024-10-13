use hmac::{digest::KeyInit, Hmac};
use jwt::{SignWithKey, VerifyWithKey};
use secrecy::{ExposeSecret, SecretString};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha384;
use time::OffsetDateTime;
use uuid::Uuid;

use super::error::AppError;

pub const ACCESS_TOKEN_LENGTH: time::Duration = time::Duration::hours(1);
pub const REFRESH_TOKEN_LENGTH: time::Duration = time::Duration::days(30);

// Create alias for HMAC-SHA256
type HmacSha384 = Hmac<Sha384>;

// All claims should have exp
pub trait Claims {
    fn exp(&self) -> i64;
}

#[derive(Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// User id
    pub sub: Uuid,
    /// Session id
    pub sid: Uuid,
    /// Expires in
    pub exp: i64,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// User id
    pub sub: Uuid,
    /// Session id
    pub sid: Uuid,
    /// Expires in
    pub exp: i64,
}

impl Claims for AccessTokenClaims {
    fn exp(&self) -> i64 {
        self.exp
    }
}

impl Claims for RefreshTokenClaims {
    fn exp(&self) -> i64 {
        self.exp
    }
}

#[derive(Clone)]
pub struct TokenManager {
    secret: HmacSha384,
}

impl TokenManager {
    pub fn new(secret: &SecretString) -> Self {
        let hmac = HmacSha384::new_from_slice(secret.expose_secret().as_bytes())
            .expect("HMAC-SHA-384 can accept any key length");

        TokenManager { secret: hmac }
    }

    #[tracing::instrument(name = "Verify token", skip_all, fields(token = ?token))]
    pub async fn verify<T: DeserializeOwned + Claims>(&self, token: &str) -> Result<T, AppError> {
        let jwt = jwt::Token::<jwt::Header, T, _>::parse_unverified(token)
            .map_err(|_| AppError::unprocessable_entity([("refresh_token", "parse")]))?;

        let jwt = jwt
            .verify_with_key(&self.secret)
            .map_err(|_| AppError::Unauthorized)?;

        let (_header, claims) = jwt.into();

        if claims.exp() < OffsetDateTime::now_utc().unix_timestamp() {
            return Err(AppError::Unauthorized);
        }

        Ok(claims)
    }

    #[tracing::instrument(name = "Genereate access token", skip_all)]
    pub fn generate_access_token(&self, user_id: Uuid, session_id: Uuid) -> String {
        let access_token = AccessTokenClaims {
            sid: session_id,
            sub: user_id,
            exp: (OffsetDateTime::now_utc() + ACCESS_TOKEN_LENGTH).unix_timestamp(),
        }
        .sign_with_key(&self.secret)
        .expect("HMAC signing should be infallible");

        return access_token;
    }

    #[tracing::instrument(name = "Genereate refresh token", skip_all)]
    pub fn generate_refresh_token(&self, user_id: Uuid, session_id: Uuid) -> String {
        let refresh_token = RefreshTokenClaims {
            sid: session_id,
            sub: user_id,
            exp: (OffsetDateTime::now_utc() + REFRESH_TOKEN_LENGTH).unix_timestamp(),
        }
        .sign_with_key(&self.secret)
        .expect("HMAC signing should be infallible");

        return refresh_token;
    }
}
