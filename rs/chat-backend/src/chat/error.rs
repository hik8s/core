use async_openai::error::OpenAIError;
use shared::connections::openai::error::ToolRequestError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatProcessingError {
    #[error("OpenAI API error: {0}")]
    OpenAI(#[from] OpenAIError),
    #[error("Tool request error: {0}")]
    ToolRequest(#[from] ToolRequestError),
}
