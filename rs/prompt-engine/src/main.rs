use error::PromptEngineServerError;
use server::initialize_prompt_engine;
use shared::tracing::setup::setup_tracing;

pub mod error;
pub mod prompt;
pub mod server;

#[rocket::main]
async fn main() -> Result<(), PromptEngineServerError> {
    setup_tracing();

    let server = initialize_prompt_engine().await?;

    server.launch().await?;
    Ok(())
}
