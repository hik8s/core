use qdrant_client::QdrantError;
use thiserror::Error;

use crate::ConfigError;

#[derive(Error, Debug)]
pub enum QdrantConnectionError {
    #[error("Qdrant client error: {0}")]
    ClientError(#[from] QdrantError),
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Serde json error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Qdrant create collection error: {0}")]
    CreateCollection(#[source] QdrantError),
    #[error("Qdrant upsert points error: {0}")]
    UpsertPoints(#[source] QdrantError),
    #[error("Qdrant set payload error: {0}")]
    SetPayload(#[source] QdrantError),
}
