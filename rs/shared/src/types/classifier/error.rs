use std::num::ParseFloatError;

use thiserror::Error;

use crate::{connections::ConfigError, RedisConnectionError};

#[derive(Error, Debug)]
pub enum ClassifierError {
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Error: {0}")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}
