use crate::{connections::ConfigError, types::classifier::state::ClassifierState};

use super::config::RedisConfig;
use redis::{Client, Commands, Connection, RedisError};
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
        self.connection
            .set(format!("{customer_id}:{key}"), serialized_value)
            .map_err(RedisConnectionError::SetError)?;
        Ok(())
    }
}
