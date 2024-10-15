use rocket::{routes, Build, Rocket};
use shared::connections::qdrant::connect::QdrantConnection;
use shared::constant::PROMPT_ENGINE_PORT;
use shared::router::rocket::build_rocket;

use crate::error::PromptEngineServerError;
use crate::prompt::route::prompt_engine;

pub async fn initialize_prompt_engine() -> Result<Rocket<Build>, PromptEngineServerError> {
    std::env::set_var("ROCKET_PORT", PROMPT_ENGINE_PORT);
    let qdrant = QdrantConnection::new().await?;
    let server = build_rocket(&vec![qdrant], routes![prompt_engine]);
    Ok(server)
}
