use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app::error::AppError;
use crate::telemetry::spawn_blocking_with_tracing;

pub struct Credentials {
    pub email: String,
    pub password_hash: SecretString,
}

#[tracing::instrument(name = "Validate credentials", skip_all)]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, AppError> {
    let mut user_id: Option<Uuid> = None;
    let mut expected_password_hash = SecretString::from(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash, reset_password)) =
        get_stored_credentials(&credentials.email, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;

        if reset_password {
            return Err(AppError::Forbidden);
        }
    }

    verify_password_hash(expected_password_hash, credentials.password_hash).await?;

    user_id.ok_or_else(|| AppError::Unauthorized)
}

#[tracing::instrument(name = "Get stored credentials", skip_all)]
async fn get_stored_credentials(
    email: &str,
    pool: &PgPool,
) -> anyhow::Result<Option<(Uuid, SecretString, bool)>> {
    let row = sqlx::query!(
        r#"
            select u.user_id, u.password_hash, u.reset_password
            from email e
            inner join "user" u using (user_id)
            where e.email = $1 and e.is_primary = true
            limit 1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
    .context("failed to retrieve stored credentials")?
    .map(|u| {
        (
            u.user_id,
            SecretString::from(u.password_hash),
            Some(true) == u.reset_password,
        )
    });

    Ok(row)
}

#[tracing::instrument(name = "Verify password hash", skip_all)]
pub async fn verify_password_hash(
    expected_password_hash: SecretString,
    candidate: SecretString,
) -> Result<(), AppError> {
    spawn_blocking_with_tracing(move || -> Result<(), AppError> {
        let hash = PasswordHash::new(expected_password_hash.expose_secret())
            .map_err(|e| anyhow::anyhow!("invalid password hash: {}", e))?;

        hash.verify_password(&[&Argon2::default()], candidate.expose_secret())
            .map_err(|e| match e {
                argon2::password_hash::Error::Password => AppError::Unauthorized,
                _ => anyhow::anyhow!("failed to verify password hash: {}", e).into(),
            })
    })
    .await
    .context("panic in verifying password hash")?
}

#[tracing::instrument(name = "Compute password hash", skip_all)]
pub async fn compute_password_hash(password: SecretString) -> Result<String, AppError> {
    spawn_blocking_with_tracing(move || -> Result<String, AppError> {
        let salt = SaltString::generate(rand::thread_rng());

        Ok(
            PasswordHash::generate(Argon2::default(), password.expose_secret(), salt.as_salt())
                .map_err(|e| anyhow::anyhow!("failed to compute password hash: {}", e))?
                .to_string(),
        )
    })
    .await
    .context("panic in computing password hash")?
}
