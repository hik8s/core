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
}

impl RedisConnection {
    pub fn new() -> Result<Self, RedisConnectionError> {
        let config = RedisConfig::new()?;
        let client = Client::open(config.get_uri())?;
        let connection = client.get_connection()?;
        let mut redis_connection = RedisConnection { connection };
        redis_connection.initialize_classifier_state()?;
        Ok(redis_connection)
    }
    pub fn initialize_classifier_state(&mut self) -> Result<(), RedisConnectionError> {
        let exists: bool = self.connection.exists(DEFAULT_STATE_KEY)?;
        if !exists {
            let initial_state = ClassifierState::new();
            self.connection.set(DEFAULT_STATE_KEY, initial_state)?;
        }
        Ok(())
    }
}
