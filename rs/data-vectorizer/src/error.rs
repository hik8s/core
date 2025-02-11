use shared::{
    connections::{
        openai::embeddings::RequestEmbeddingError, qdrant::error::QdrantConnectionError,
        redis::connect::RedisConnectionError,
    },
    fluvio::{FluvioConnectionError, OffsetError},
};
use std::str::Utf8Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataVectorizationError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Fluvio offset error: {0}")]
    FluvioOffsetError(#[from] OffsetError),
    #[error("Qdrant connection error: {0}")]
    QdrantConnectionError(#[from] QdrantConnectionError),
    #[error("Redis get error: {0}")]
    RedisGet(#[source] RedisConnectionError),
    #[error("Redis set error: {0}")]
    RedisSet(#[source] RedisConnectionError),
    #[error("Redis init error: {0}")]
    RedisInit(#[source] RedisConnectionError),
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
    #[error("OpenAI API error: {0}")]
    RequestEmbedding(#[from] RequestEmbeddingError),
    #[error("Error converting to qdrant points: {0}")]
    QdrantPointsConversion(#[source] serde_json::Error),
    #[error("Class deserialization error: {0}")]
    ClassDeserialization(#[source] serde_json::Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Serialization error: {0}")]
    SerializationError(#[source] serde_json::Error),
    #[error("Deserialization error: {0}")]
    DeserializationError(#[source] serde_json::Error),
    #[error("Resource misses field: {0}")]
    MissingField(String),
}
