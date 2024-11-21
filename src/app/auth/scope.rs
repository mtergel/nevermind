use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::error::AppError;

#[derive(Clone)]
pub struct UserScopes {
    pub scopes: Vec<AppPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "permission")]
pub enum AppPermission {
    #[sqlx(rename = "user.view")]
    #[serde(rename = "user.view")]
    UserView,
}

impl std::fmt::Display for AppPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let scope_str = match self {
            AppPermission::UserView => "user.view",
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

#[tracing::instrument(name = "Get user scopes", skip_all)]
pub async fn get_scopes(user_id: Uuid, pool: &PgPool) -> Result<UserScopes, AppError> {
    let scopes = sqlx::query_scalar!(
        r#"
            select rp.permission as "permission!: AppPermission"
            from user_role ur
            join role_permission rp
                on ur.role = rp.role
            where ur.user_id = $1 
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(UserScopes { scopes })
}
