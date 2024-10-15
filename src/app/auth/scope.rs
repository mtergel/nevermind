use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app::error::AppError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Scope {
    #[serde(rename = "public")]
    Public,

    #[serde(rename = "write:user")]
    WriteUser,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let scope_str = match self {
            Scope::Public => "public",
            Scope::WriteUser => "write:user",
        };
        write!(f, "{}", scope_str)
    }
}

const UNVERIFIED_USER_SCOPES: &[Scope] = &[Scope::Public];
const USER_SCOPES: &[Scope] = &[Scope::WriteUser];

pub struct UserScopes {
    pub scopes: Vec<Scope>,
}

impl std::fmt::Display for UserScopes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scopes_str = self
            .scopes
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        write!(f, "{}", scopes_str)
    }
}

#[tracing::instrument(name = "Get user scopes", skip_all)]
pub async fn get_scopes(user_id: Uuid, pool: &PgPool) -> Result<UserScopes, AppError> {
    let verified = sqlx::query_scalar!(
        r#"
            select verified
            from email
            where user_id = $1 and is_primary = true
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("failed to retrieve permissions")
    .unwrap();

    let mut scopes: Vec<Scope> = UNVERIFIED_USER_SCOPES.to_vec();

    if verified {
        scopes.append(&mut USER_SCOPES.to_vec());
    }

    Ok(UserScopes { scopes })
}
