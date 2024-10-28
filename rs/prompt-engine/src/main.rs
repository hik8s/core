use prompt_engine::{error::PromptEngineServerError, server::initialize_prompt_engine};
use shared::tracing::setup::setup_tracing;

#[rocket::main]
async fn main() -> Result<(), PromptEngineServerError> {
    setup_tracing(false);

    let server = initialize_prompt_engine().await?;

    server.launch().await?;
    Ok(())
}
