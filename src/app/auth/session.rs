use anyhow::Context;
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::token::{TokenManager, ACCESS_TOKEN_LENGTH, REFRESH_TOKEN_LENGTH};
use crate::app::error::AppError;

pub struct Session {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionMetadata {
    pub device_name: Option<String>,
    pub ip: Option<String>,
    pub last_accessed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionData {
    pub metadata: SessionMetadata,
    #[schema(value_type = String)]
    pub session_id: Uuid,
    pub refresh_token: String,
}

pub struct Tokens {
    pub access_token: String,
    pub expires_in: u64,

    pub refresh_token: String,
}

impl Session {
    fn get_user_session_key(&self) -> String {
        format!("user:{}:session_id:{}", self.user_id, self.session_id)
    }

    #[tracing::instrument(name = "Create new session", skip_all)]
    pub fn new(user_id: Uuid) -> Self {
        Session {
            user_id,
            session_id: Uuid::new_v4(),
        }
    }

    #[tracing::instrument(name = "Get session data", skip_all)]
    pub async fn get_data(&self, client: &Client) -> Result<SessionData, AppError> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let res: Option<String> = redis::cmd("JSON.GET")
            .arg(self.get_user_session_key())
            .arg("$")
            .query_async(&mut conn)
            .await
            .context("failed to get value from redis")?;

        match res {
            Some(raw_data) => {
                let data: Vec<SessionData> =
                    serde_json::from_str(&raw_data).context("failed to parse redis value")?;

                if !data.is_empty() {
                    return Ok(data[0].clone());
                }

                Err(AppError::Unauthorized)
            }
            None => Err(AppError::Unauthorized),
        }
    }

    #[tracing::instrument(name = "List active sessions", skip_all)]
    pub async fn list_sessions(&self, client: &Client) -> anyhow::Result<Vec<SessionData>> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let mut keys: Vec<String> = Vec::new();

        // Get keys
        let pattern = format!("user:{}:session_id:*", self.user_id);
        let mut iter: redis::AsyncIter<String> = conn
            .scan_match(pattern)
            .await
            .expect("failed to scan iterate to redis");

        while let Some(key) = iter.next_item().await {
            keys.push(key);
        }

        drop(iter);

        let values: Vec<Option<String>> = redis::cmd("JSON.MGET")
            .arg(&keys)
            .arg("$")
            .query_async(&mut conn)
            .await
            .context("failed to get values from redis")?;

        let filtered_sessions: Vec<String> = values.into_iter().filter_map(|x| x).collect();
        let sessions: anyhow::Result<Vec<SessionData>> = filtered_sessions
            .into_iter()
            .map(|x| {
                let data: Vec<SessionData> =
                    serde_json::from_str(&x).context("failed to parse redis value")?;

                data.get(0).cloned().context("no session data found")
            })
            .collect();

        match sessions {
            Ok(sessions) => Ok(sessions),
            Err(e) => return Err(e),
        }
    }

    #[tracing::instrument(name = "Insert session into redis", skip_all, fields(metadata = ?metadata))]
    pub async fn insert(
        &self,
        metadata: SessionMetadata,
        client: &Client,
        token_manager: &TokenManager,
    ) -> Result<Tokens, anyhow::Error> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let access_token = token_manager.generate_access_token(self.user_id, self.session_id);
        let refresh_token = token_manager.generate_refresh_token(self.user_id, self.session_id);

        let data = SessionData {
            metadata,
            session_id: self.session_id,
            refresh_token: refresh_token.clone(),
        };

        // Insert into redis
        redis::pipe()
            .atomic()
            .cmd("JSON.SET")
            .arg(self.get_user_session_key())
            .arg("$")
            .arg(serde_json::to_string(&data).unwrap())
            .ignore()
            .cmd("EXPIRE")
            .arg(self.get_user_session_key())
            .arg(REFRESH_TOKEN_LENGTH.whole_seconds() as u64)
            .ignore()
            .exec_async(&mut conn)
            .await?;

        Ok(Tokens {
            access_token,
            refresh_token,
            expires_in: ACCESS_TOKEN_LENGTH.whole_seconds() as u64,
        })
    }

    #[tracing::instrument(name = "Renew session into redis", skip_all, fields(metadata = ?metadata))]
    pub async fn renew(
        &self,
        metadata: SessionMetadata,
        client: &Client,
        token_manager: &TokenManager,
    ) -> Result<Tokens, anyhow::Error> {
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .context("failed to connect to redis")
            .unwrap();

        let access_token = token_manager.generate_access_token(self.user_id, self.session_id);
        let refresh_token = token_manager.generate_refresh_token(self.user_id, self.session_id);

        let data = SessionData {
            metadata,
            session_id: self.session_id,
            refresh_token: refresh_token.clone(),
        };

        // Insert into redis
        redis::pipe()
            .atomic()
            .cmd("JSON.SET")
            .arg(self.get_user_session_key())
            .arg("$")
            .arg(serde_json::to_string(&data).unwrap())
            .ignore()
            .cmd("EXPIRE")
            .arg(self.get_user_session_key())
            .arg(REFRESH_TOKEN_LENGTH.whole_seconds() as u64)
            .ignore()
            .exec_async(&mut conn)
            .await?;

        Ok(Tokens {
            access_token,
            refresh_token,
            expires_in: ACCESS_TOKEN_LENGTH.whole_seconds() as u64,
        })
    }
}
