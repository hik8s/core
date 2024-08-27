use rocket::{http::Status, response::Responder, Request, Response};
use shared::connections::qdrant::error::QdrantConnectionError;
use std::io::Cursor;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum PromptEngineError {
    #[error("Qdrant error: {0}")]
    QdrantConnectionError(#[from] QdrantConnectionError),
}

impl From<PromptEngineError> for Status {
    fn from(error: PromptEngineError) -> Self {
        match error {
            PromptEngineError::QdrantConnectionError(e) => {
                info!("Qdrant error: {:?}", e);
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
