use std::{collections::HashSet, str::FromStr};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app::error::AppError;

#[derive(Clone)]
pub struct UserScopes {
    pub scopes: Vec<AppPermission>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "app_permission")]
pub enum AppPermission {
    #[sqlx(rename = "user.create")]
    #[serde(rename = "user.create")]
    UserCreate,
    #[sqlx(rename = "user.read")]
    #[serde(rename = "user.read")]
    UserRead,
    #[sqlx(rename = "user.update")]
    #[serde(rename = "user.update")]
    UserUpdate,
    #[sqlx(rename = "user.delete")]
    #[serde(rename = "user.delete")]
    UserDelete,
}

impl std::fmt::Display for AppPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let scope_str = match self {
            AppPermission::UserCreate => "user.create",
            AppPermission::UserRead => "user.read",
            AppPermission::UserUpdate => "user.update",
            AppPermission::UserDelete => "user.delete",
        };
        write!(f, "{}", scope_str)
    }
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

impl FromStr for AppPermission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user.create" => Ok(Self::UserCreate),
            "user.read" => Ok(Self::UserRead),
            "user.update" => Ok(Self::UserUpdate),
            "user.delete" => Ok(Self::UserDelete),
            _ => Err(format!("Unknown permission: {}", s)),
        }
    }
}

impl AppPermission {
    /// Converts a space-separated string of permissions into a HashSet of AppPermission.
    #[tracing::instrument(name = "Parse scope string")]
    pub fn parse_permissions(permission_str: &str) -> Result<HashSet<AppPermission>, String> {
        permission_str
            .split_whitespace()
            .map(str::to_string)
            .map(|perm| AppPermission::from_str(&perm))
            .collect()
    }
}

#[tracing::instrument(name = "Get user scopes", skip_all)]
pub async fn get_scopes(user_id: Uuid, pool: &PgPool) -> Result<UserScopes, AppError> {
    let scopes = sqlx::query_scalar!(
        r#"
            select permission as "permission!: AppPermission"
            from user_permission
            where user_id = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(UserScopes { scopes })
}
