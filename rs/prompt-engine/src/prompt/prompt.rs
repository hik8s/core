use rocket::{post, serde::json::Json};
use shared::{
    connections::{
        get_db_name,
        prompt_engine::connect::AugmentationRequest,
        qdrant::connect::{namespace, not_namespace, QdrantConnection},
    },
    openai::embed::request_embedding,
};

use super::error::PromptEngineError;
use super::{augmentation::create_augmented_prompt, understanding::request_input_understanding};

#[post("/prompt", format = "json", data = "<payload>")]
pub async fn prompt_engine(
    qdrant: QdrantConnection,
    payload: Json<AugmentationRequest>,
) -> Result<String, PromptEngineError> {
    let request = payload.into_inner();
    let understanding = request_input_understanding(&request.user_message).await?;
    let search_prompt = understanding.create_search_prompt(&request.user_message);

    let array = request_embedding(&vec![search_prompt]).await?[0];

    let db_name = get_db_name(&request.client_id);

    let kube_system = qdrant
        .search_classes(&db_name, array, namespace("kube-system"), 15)
        .await?;
    let namespace_filter = match understanding.namespace {
        Some(ref ns) => namespace(&ns),
        None => not_namespace("kube-system"),
    };
    let other_namespace = qdrant
        .search_classes(&db_name, array, namespace_filter, 15)
        .await?;
    let augment_prompt = create_augmented_prompt(
        &request.user_message,
        kube_system,
        other_namespace,
        &understanding,
    );
    Ok(augment_prompt)
}
