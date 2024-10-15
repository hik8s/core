use chat::route::chat_completion;
use rocket::{main, routes};
use shared::{
    connections::prompt_engine::connect::{PromptEngineConnection, PromptEngineError},
    constant::CHAT_BACKEND_PORT,
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
}

#[main]
async fn main() -> Result<(), ChatBackendError> {
    setup_tracing(false);
    std::env::set_var("ROCKET_PORT", CHAT_BACKEND_PORT);

    let prompt_engine = PromptEngineConnection::new()?;

    let rocket = build_rocket(&vec![prompt_engine], routes![chat_completion]);

    rocket.launch().await?;
    Ok(())
}
