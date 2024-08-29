use rocket::{http::Status, response::Responder, Request, Response};
use shared::{
    connections::qdrant::error::QdrantConnectionError, openai::embed::RequestEmbeddingError,
};
use std::io::Cursor;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum PromptEngineError {
    #[error("Qdrant error: {0}")]
    QdrantConnectionError(#[from] QdrantConnectionError),
    #[error("OpenAI API error: {0}")]
    RequestEmbeddingError(#[from] RequestEmbeddingError),
}

impl From<PromptEngineError> for Status {
    fn from(error: PromptEngineError) -> Self {
        match error {
            PromptEngineError::QdrantConnectionError(e) => {
                error!("Qdrant error: {:?}", e);
                Status::InternalServerError
            }
            PromptEngineError::RequestEmbeddingError(e) => {
                error!("OpenAI API error: {:?}", e);
                Status::InternalServerError
            }
        }
    }
}

impl<'r> Responder<'r, 'static> for PromptEngineError {
    fn respond_to(self, _: &'r Request<'_>) -> Result<Response<'static>, Status> {
        let body = format!("{}", self);
        let status = Status::from(self);
        Response::build()
            .status(status)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}
