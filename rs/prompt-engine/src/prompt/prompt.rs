use rocket::{post, serde::json::Json};
use shared::{
    connections::{
        get_db_name,
        prompt_engine::connect::AugmentationRequest,
        qdrant::connect::{namespace, not_namespace, QdrantConnection},
    },
    openai::embed::request_embedding,
};

use super::augmentation::create_augmented_prompt;
use super::error::PromptEngineError;

#[post("/prompt", format = "json", data = "<payload>")]
pub async fn prompt_engine(
    qdrant: QdrantConnection,
    payload: Json<AugmentationRequest>,
) -> Result<String, PromptEngineError> {
    let request = payload.into_inner();

    let array = request_embedding(&vec![request.user_message.clone()]).await?[0];

    let db_name = get_db_name(&request.client_id);
    let kube_system = qdrant
        .search_classes(&db_name, array, namespace("kube-system"), 15)
        .await?;
    let other_namespace = qdrant
        .search_classes(&db_name, array, not_namespace("kube-system"), 20)
        .await?;

    let augment_prompt =
        create_augmented_prompt(&request.user_message, kube_system, other_namespace);
    Ok(augment_prompt)
}
