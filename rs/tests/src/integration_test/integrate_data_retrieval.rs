#[cfg(test)]
mod tests {
    use data_vectorizer::vectorize::vectorize_classes;
    use prompt_engine::{
        prompt::test_prompt::{get_scenario_data, ClusterTestScenario},
        server::initialize_prompt_engine,
    };
    use rocket::http::ContentType;
    use rstest::rstest;
    use shared::{
        connections::{
            prompt_engine::connect::AugmentationRequest, qdrant::connect::QdrantConnection,
        },
        constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
        get_db_name, get_env_var,
        mock::rocket::get_test_client,
        tracing::setup::setup_tracing,
        types::tokenizer::Tokenizer,
        utils::ratelimit::RateLimiter,
    };
    use tracing::info;

    #[tokio::test]
    #[rstest]
    #[case(ClusterTestScenario::PodKillOutOffMemory)]
    async fn test_retrieval_integration(#[case] test_scenario: ClusterTestScenario) {
        // Prep
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();
        let tokenizer = Tokenizer::new().unwrap();
        let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);

        // Data ingestion
        let scenario_data = get_scenario_data(&test_scenario);
        let points =
            vectorize_classes(&scenario_data.vectorized_classes, &tokenizer, &rate_limiter)
                .await
                .unwrap()
                .0;
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
        let db_name = get_db_name(&client_id);
        qdrant.upsert_points(points, &db_name).await.unwrap();

        // Data retrieval
        let server = initialize_prompt_engine().await.unwrap();
        let client = get_test_client(server).await.unwrap();

        let prompt = format!(
            "What is going on in namespace {} seems to restart. What is the issue here?",
            scenario_data.vectorized_classes[0].namespace
        );
        let body: String = AugmentationRequest::new(&prompt, &client_id)
            .try_into()
            .unwrap();

        let response = client
            .post("/prompt")
            .header(ContentType::JSON)
            .body(body)
            .dispatch()
            .await;

        // Add assertions to check the response
        assert_eq!(response.status().code, 200);
        let body = response.into_string().await.unwrap();
        info!("Response: {}", body);
        assert!(body.contains("OOMKilled Exit Code 137"));
    }
}
