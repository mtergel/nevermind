use anyhow::Context;
use regex::Regex;
use secrecy::SecretString;
use sqlx::PgPool;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::app::{auth::password::verify_password_hash, error::AppError};

pub static USERNAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{3,32}$").unwrap());

#[tracing::instrument(name = "Password required API, validating")]
pub async fn validate_password(
    candidate_pw: SecretString,
    user_id: &Uuid,
    pool: &PgPool,
) -> Result<(), AppError> {
    let user_pw_hash = sqlx::query_scalar!(
        r#"
            select password_hash 
            from "user"
            where user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("failed to retrieve stored credentials.")?;

    verify_password_hash(SecretString::from(user_pw_hash), candidate_pw).await
}
