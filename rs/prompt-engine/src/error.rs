use shared::connections::qdrant::error::QdrantConnectionError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PromptEngineServerError {
    #[error("Qdrant connection error: {0}")]
    QdrantConnectionError(#[from] QdrantConnectionError),
    #[error("Rocket error: {0}")]
    RocketError(#[from] rocket::Error),
}
