use async_openai::error::OpenAIError;
use shared::{connections::openai::error::ToolRequestError, QdrantConnectionError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatProcessingError {
    #[error("OpenAI API error: {0}")]
    OpenAI(#[from] OpenAIError),
    #[error("Qdrant connection error: {0}")]
    Qdrant(#[from] QdrantConnectionError),
    #[error("Tool execution error: {0}")]
    ToolExecution(String),
    #[error("Message stream error: {0}")]
    MessageStream(String),
    #[error("Tool request error: {0}")]
    ToolRequest(#[from] ToolRequestError),
}
