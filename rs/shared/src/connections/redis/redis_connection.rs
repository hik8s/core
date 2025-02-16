use std::time::Duration;

use crate::{
    connections::{dbname::DbName, ConfigError},
    types::classifier::state::ClassifierState,
};

use super::config::RedisConfig;
use redis::{Client, Commands, Connection, FromRedisValue, RedisError, ToRedisArgs};
use serde::Serialize;
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
    #[error("Redis init error: {0}")]
    RedisInit(#[source] RedisError),
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
        let client = Client::open(config.get_uri()).map_err(RedisConnectionError::RedisInit)?;
        let connection = client
            .get_connection()
            .map_err(RedisConnectionError::RedisInit)?;
        let redis_connection = RedisConnection { connection };
        Ok(redis_connection)
    }

    pub fn key(&self, db: DbName, customer_id: &str, kind: Option<&str>, uid: &str) -> String {
        if let Some(kind) = kind {
            format!("{}_{}_{}", db.id(customer_id), kind, uid)
        } else {
            format!("{}_{}", db.id(customer_id), uid)
        }
    }

    pub fn get(&mut self, key: &str) -> Result<ClassifierState, RedisConnectionError> {
        let exists: bool = self.connection.exists(key)?;
        match exists {
            true => {
                let state: ClassifierState = self
                    .connection
                    .get(key)
                    .map_err(RedisConnectionError::GetError)?;
                Ok(state)
            }
            false => {
                info!("Creating new state for key: {}", key);
                Ok(ClassifierState { classes: vec![] })
            }
        }
    }

    pub fn set(&mut self, key: &str, value: ClassifierState) -> Result<(), RedisConnectionError> {
        let serialized_value: String = serde_json::to_string(&value).unwrap();
        let _res: () = self
            .connection
            .set(key, serialized_value)
            .map_err(RedisConnectionError::SetError)?;
        Ok(())
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
    pub async fn get_with_retry<T>(&mut self, key: &str) -> Result<Option<T>, RedisConnectionError>
    where
        T: FromRedisValue + serde::de::DeserializeOwned,
    {
        if !self.connection.exists(key)? {
            return Ok(None);
        }
        self.retry(|conn| conn.connection.get(key), 3).await
    }
    pub async fn set_with_retry<T>(
        &mut self,
        key: &str,
        value: &T,
    ) -> Result<(), RedisConnectionError>
    where
        T: Serialize + ToRedisArgs,
    {
        self.retry(|conn| conn.connection.set(key, value), 3).await
    }
}
