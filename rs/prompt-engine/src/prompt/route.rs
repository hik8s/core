use rocket::{post, serde::json::Json};
use shared::{
    connections::{
        db_name::get_db_name,
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

    let array = request_embedding(&request.user_message).await?;

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

#[cfg(test)]
mod tests {
    use rocket::http::ContentType;
    use shared::{
        connections::prompt_engine::connect::AugmentationRequest, get_env_var,
        mock::rocket::get_test_client, tracing::setup::setup_tracing,
    };

    use crate::server::initialize_prompt_engine;

    #[tokio::test]
    async fn test_prompt() {
        setup_tracing();
        let server = initialize_prompt_engine().await.unwrap();
        let client = get_test_client(server).await.unwrap();

        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
        let body: String = AugmentationRequest::new("Test message", &client_id)
            .try_into()
            .unwrap();

        let response = client
            .post("/prompt")
            .header(ContentType::JSON)
            .body(body)
            .dispatch()
            .await;

        // Add assertions to check the response
        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body = response.into_string().await.unwrap();
        assert!(body.len() > 500);
    }
}
