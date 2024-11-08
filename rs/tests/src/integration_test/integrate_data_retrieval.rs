#[cfg(test)]
mod tests {
    use data_vectorizer::vectorize::vectorize_classes;
    use prompt_engine::prompt::test_prompt::{get_scenario_data, ClusterTestScenario};
    use rstest::rstest;
    use shared::{
        connections::{
            openai::tools::{LogRetrievalArgs, Tool},
            qdrant::connect::QdrantConnection,
        },
        constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
        get_db_name, get_env_var,
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

        let prompt = format!(
            "What is going on in namespace {} seems to restart. What is the issue here?",
            scenario_data.vectorized_classes[0].namespace
        );

        let tool = Tool::LogRetrieval(LogRetrievalArgs::default());
        let result = tool.request(&qdrant, &prompt, &client_id).await.unwrap();

        info!(result);
        assert!(result.contains("OOMKilled Exit Code 137"));
    }
}
