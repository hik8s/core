use rocket::post;
use shared::{
    connections::qdrant::connect::{namespace, not_namespace, QdrantConnection},
    openai::embed::request_embedding,
};

use super::augmentation::create_augmented_prompt;
use super::error::PromptEngineError;

#[post("/prompt", data = "<user_message>")]
pub async fn prompt_engine(
    qdrant: QdrantConnection,
    user_message: &str,
) -> Result<String, PromptEngineError> {
    let array = request_embedding(user_message.to_string()).await?;

    let kube_system = qdrant
        .search_classes(array, namespace("kube-system"), 15)
        .await?;
    let other_namespace = qdrant
        .search_classes(array, not_namespace("kube-system"), 20)
        .await?;

    let augment_prompt = create_augmented_prompt(user_message, kube_system, other_namespace);
    Ok(augment_prompt)
}

#[cfg(test)]
mod tests {
    use rocket::routes;
    use shared::{
        connections::qdrant::connect::QdrantConnection, constant::QDRANT_COLLECTION_LOG,
        mock::rocket::rocket_test_client, tracing::setup::setup_tracing,
    };
    use tracing::info;

    use crate::prompt::route::prompt_engine;

    #[tokio::test]
    async fn test_prompt() {
        setup_tracing();
        let qdrant = QdrantConnection::new(QDRANT_COLLECTION_LOG.to_owned())
            .await
            .unwrap();
        let client = rocket_test_client(&vec![qdrant], routes![prompt_engine])
            .await
            .unwrap();

        let user_message = "Test message";
        let response = client.post("/prompt").body(user_message).dispatch().await;

        // Add assertions to check the response
        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body = response.into_string().await.unwrap();
        info!("Response body length: {}", body.len());
        assert!(body.len() > 500);
    }
}
