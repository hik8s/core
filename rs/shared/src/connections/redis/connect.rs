use crate::{connections::error::ConfigError, types::classification::state::ClassifierState};

use super::config::RedisConfig;
use redis::{Client, Commands, Connection, RedisError};
use thiserror::Error;

const DEFAULT_STATE_KEY: &str = "classifier_state";
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
}

impl RedisConnection {
    pub fn new() -> Result<Self, RedisConnectionError> {
        let config = RedisConfig::new()?;
        let client = Client::open(config.get_uri())?;
        let connection = client.get_connection()?;
        let redis_connection = RedisConnection { connection };
        Ok(redis_connection)
    }

    pub fn get(&mut self, key: &str) -> Result<ClassifierState, RedisConnectionError> {
        let exists: bool = self
            .connection
            .exists(format!("{DEFAULT_STATE_KEY}:{key}"))?;
        match exists {
            true => {
                let state: ClassifierState = self
                    .connection
                    .get(format!("{DEFAULT_STATE_KEY}:{key}"))
                    .map_err(RedisConnectionError::GetError)?;
                Ok(state)
            }
            false => {
                tracing::info!("Creating new state for key: {}", key);
                Ok(ClassifierState { classes: vec![] })
            }
        }
    }

    pub fn set(&mut self, key: &str, value: ClassifierState) -> Result<(), RedisConnectionError> {
        let serialized_value: String = serde_json::to_string(&value).unwrap();
        self.connection
            .set(format!("{DEFAULT_STATE_KEY}:{key}"), serialized_value)
            .map_err(RedisConnectionError::SetError)?;
        Ok(())
    }
}
