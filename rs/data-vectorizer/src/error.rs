use shared::{
    connections::openai::embeddings::RequestEmbeddingError,
    connections::qdrant::error::QdrantConnectionError,
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
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
    #[error("OpenAI API error: {0}")]
    OpenAIError(#[from] RequestEmbeddingError),
    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Deserialization error: {0}")]
    DeserializationError(#[source] serde_json::Error),
}
