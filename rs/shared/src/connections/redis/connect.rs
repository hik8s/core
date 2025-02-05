use std::time::Duration;

use crate::{connections::ConfigError, types::classifier::state::ClassifierState};

use super::config::RedisConfig;
use redis::{Client, Commands, Connection, FromRedisValue, RedisError};
use thiserror::Error;
use tracing::info;

pub struct RedisConnection {
    pub connection: Connection,
}
#[derive(Error, Debug)]
pub enum RedisConnectionError {
    #[error("Failed to create DbConfig: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Redis error: {0}")]
    RedisError(#[from] RedisError),
    #[error("Failed to get value from Redis: {0}")]
    GetError(#[source] RedisError),
    #[error("Failed to set value in Redis: {0}")]
    SetError(#[source] RedisError),
    #[error("Max retries reached, failed after {0} attempts: {1}")]
    RetryError(u32, RedisError),
}

impl RedisConnection {
    pub fn new() -> Result<Self, RedisConnectionError> {
        let config = RedisConfig::new()?;
        let client = Client::open(config.get_uri())?;
        let connection = client.get_connection()?;
        let redis_connection = RedisConnection { connection };
        Ok(redis_connection)
    }

    pub fn get(
        &mut self,
        customer_id: &str,
        key: &str,
    ) -> Result<ClassifierState, RedisConnectionError> {
        let identifier = format!("{customer_id}:{key}");
        let exists: bool = self.connection.exists(&identifier)?;
        match exists {
            true => {
                let state: ClassifierState = self
                    .connection
                    .get(&identifier)
                    .map_err(RedisConnectionError::GetError)?;
                Ok(state)
            }
            false => {
                info!("Creating new state for key: {}", identifier);
                Ok(ClassifierState { classes: vec![] })
            }
        }
    }

    pub fn set(
        &mut self,
        customer_id: &str,
        key: &str,
        value: ClassifierState,
    ) -> Result<(), RedisConnectionError> {
        let serialized_value: String = serde_json::to_string(&value).unwrap();
        let _res: () = self
            .connection
            .set(format!("{customer_id}:{key}"), serialized_value)
            .map_err(RedisConnectionError::SetError)?;
        Ok(())
    }

    pub async fn get_json(
        &mut self,
        customer_id: &str,
        key: &str,
    ) -> Result<Option<String>, RedisConnectionError> {
        self.get_with_retry::<String>(customer_id, key).await
    }

    pub async fn retry<T, F>(
        &mut self,
        mut f: F,
        max_retries: u32,
    ) -> Result<T, RedisConnectionError>
    where
        F: FnMut(&mut Self) -> Result<T, RedisError>,
    {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(100);

        loop {
            match f(self) {
                Ok(value) => return Ok(value),
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_retries {
                        return Err(RedisConnectionError::RetryError(attempts, e));
                    }
                    tracing::warn!("Attempt {} failed: {:?}, retrying...", attempts, e);
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }
    }
    pub async fn get_with_retry<T>(
        &mut self,
        customer_id: &str,
        key: &str,
    ) -> Result<Option<T>, RedisConnectionError>
    where
        T: FromRedisValue + serde::de::DeserializeOwned,
    {
        self.retry(
            |conn| {
                let identifier = format!("{customer_id}:{key}");
                let exists: bool = conn.connection.exists(&identifier)?;
                match exists {
                    true => {
                        let value: T = conn.connection.get(&identifier)?;
                        Ok(Some(value))
                    }
                    false => Ok(None),
                }
            },
            3,
        )
        .await
    }
}
