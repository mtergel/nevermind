use anyhow::Context;
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::{
    error::AppError,
    token::{TokenManager, ACCESS_TOKEN_LENGTH, REFRESH_TOKEN_LENGTH},
};

pub struct Session {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub device_name: Option<String>,
    pub ip: Option<String>,
    pub last_accessed: String,
}

#[derive(Serialize, Deserialize)]
pub struct SessionData {
    pub metadata: SessionMetadata,
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

        let res: Option<String> = conn
            .get(self.get_user_session_key())
            .await
            .map_err(|e| AppError::Anyhow(e.into()))?;

        match res {
            Some(res) => {
                let data: SessionData =
                    serde_json::from_str(&res).context("failed to parse session json")?;

                Ok(data)
            }

            None => Err(AppError::Unauthorized),
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
