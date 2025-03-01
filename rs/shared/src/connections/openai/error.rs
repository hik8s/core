use async_openai::error::OpenAIError;
use thiserror::Error;

use crate::{GreptimeConnectionError, QdrantConnectionError};

#[derive(Error, Debug)]
pub enum ToolRequestError {
    #[error("Qdrant vector DB error: {0}")]
    Qdrant(#[from] QdrantConnectionError),
    #[error("Greptime error: {0}")]
    Greptime(#[from] GreptimeConnectionError),
    #[error("OpenAI API error: {0}")]
    OpenAI(#[from] OpenAIError),
    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),
}
