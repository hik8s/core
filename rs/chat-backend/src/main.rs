use chat::route::chat_completion;
use rocket::{main, routes};
use shared::{
    connections::{
        prompt_engine::connect::{PromptEngineConnection, PromptEngineError},
        qdrant::{connect::QdrantConnection, error::QdrantConnectionError},
    },
    constant::{CHAT_BACKEND_PORT, QDRANT_COLLECTION_LOG},
    router::rocket::build_rocket,
    tracing::setup::setup_tracing,
};

pub mod chat;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatBackendError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] PromptEngineError),
    #[error("Rocket error: {0}")]
    RocketError(#[from] rocket::Error),
    #[error("Qdrant error: {0}")]
    QdrantConnectionError(#[from] QdrantConnectionError),
}

#[main]
async fn main() -> Result<(), ChatBackendError> {
    setup_tracing();
    std::env::set_var("ROCKET_PORT", CHAT_BACKEND_PORT);

    let client = PromptEngineConnection::new()?;

    let qdrant_connection = QdrantConnection::new(QDRANT_COLLECTION_LOG.to_owned()).await?;

    let rocket = build_rocket(&vec![qdrant_connection], routes![chat_completion]).attach(client);

    rocket.launch().await?;
    Ok(())
}
