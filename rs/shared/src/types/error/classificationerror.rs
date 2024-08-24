use crate::connections::redis::connect::RedisConnectionError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClassificationError {
    #[error("Comparison error")]
    ComparisonError,
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
}
