use chat_backend::chat::route::chat_completion;
use rocket::{main, routes};
use shared::{
    constant::CHAT_BACKEND_PORT, router::rocket::build_rocket, setup_tracing, QdrantConnection,
    QdrantConnectionError,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatBackendError {
    #[error("Qdrant error: {0}")]
    QdrantError(#[from] QdrantConnectionError),
    #[error("Rocket error: {0}")]
    RocketError(#[from] rocket::Error),
}

#[main]
async fn main() -> Result<(), ChatBackendError> {
    setup_tracing(false);
    std::env::set_var("ROCKET_PORT", CHAT_BACKEND_PORT);

    let qdrant = QdrantConnection::new().await?;

    let rocket = build_rocket(&[qdrant], routes![chat_completion]);

    rocket.launch().await?;
    Ok(())
}
