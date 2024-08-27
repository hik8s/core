use rocket::post;
use shared::{connections::qdrant::connect::QdrantConnection, openai::embed::request_embedding};
use tracing::info;

use super::error::PromptEngineError;

#[post("/prompt", data = "<user_message>")]
pub async fn prompt_engine(
    qdrant_connection: QdrantConnection,
    user_message: &str,
) -> Result<String, PromptEngineError> {
    info!("## We got this user message: '{user_message}'\n\n",);
    let vector = request_embedding(user_message.to_string()).await?;

    Ok("success".to_string())
}
