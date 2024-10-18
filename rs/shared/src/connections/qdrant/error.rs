use qdrant_client::QdrantError;
use thiserror::Error;

use crate::connections::ConfigError;

#[derive(Error, Debug)]
pub enum QdrantConnectionError {
    #[error("Qdrant client error: {0}")]
    ClientError(#[from] QdrantError),
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Serde json error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
}
