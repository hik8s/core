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
    #[error("Missing tool call ID")]
    MissingId,
    #[error("Invalid function call: {0}")]
    InvalidFunction(String),
    #[error("No tool calls found")]
    EmptyToolCalls,
    #[error("Unknown tool called: '{0}'")]
    UnknownTool(String),
}
