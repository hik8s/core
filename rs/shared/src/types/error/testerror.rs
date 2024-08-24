use crate::connections::{error::ConfigError, redis::connect::RedisConnectionError};

use super::classificationerror::ClassificationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Classification error: {0}")]
    ClassificationError(#[from] ClassificationError),
    #[error("Failed to initialize config: {0}")]
    ConfigError(#[from] ConfigError),
}
