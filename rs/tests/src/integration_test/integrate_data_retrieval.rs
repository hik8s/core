#[cfg(test)]
mod tests {
    use data_vectorizer::vectorize::vectorize_classes;
    use rstest::rstest;
    use shared::{
        connections::{
            openai::tools::{LogRetrievalArgs, Tool},
            qdrant::{connect::QdrantConnection, error::QdrantConnectionError},
        },
        constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
        get_db_name, get_env_var,
        testdata::{UserTest, UserTestData},
        tracing::setup::setup_tracing,
        types::tokenizer::Tokenizer,
        utils::ratelimit::RateLimiter,
    };
    use tracing::info;

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::PodKillOutOffMemory))]
    async fn test_retrieval_integration(
        #[case] testdata: UserTestData,
    ) -> Result<(), QdrantConnectionError> {
        // Prep
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();
        let tokenizer = Tokenizer::new().unwrap();
        let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);

        // Data ingestion
        let points = vectorize_classes(&vec![testdata.class.clone()], &tokenizer, &rate_limiter)
            .await
            .unwrap()
            .0;
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
        let db_name = get_db_name(&client_id);
        qdrant.upsert_points(points, &db_name).await.unwrap();

        let tool = Tool::LogRetrieval(LogRetrievalArgs::new(&testdata));
        let result = tool.request(&qdrant, &testdata.prompt, &client_id).await?;

        info!(result);
        assert!(result.contains("OOMKilled Exit Code 137"));
        Ok(())
    }
}
