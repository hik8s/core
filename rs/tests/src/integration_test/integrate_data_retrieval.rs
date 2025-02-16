#[cfg(test)]
mod tests {
    use data_vectorizer::vectorize::vectorizer::vectorize_class_batch;
    use rstest::rstest;
    use shared::{
        connections::{
            dbname::DbName,
            openai::tools::{LogRetrievalArgs, Tool},
        },
        constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
        get_env_var,
        testdata::{UserTest, UserTestData},
        tracing::setup::setup_tracing,
        types::tokenizer::Tokenizer,
        utils::ratelimit::RateLimiter,
        QdrantConnection, QdrantConnectionError,
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
        let points = vectorize_class_batch(&[testdata.class.clone()], &tokenizer, &rate_limiter)
            .await
            .unwrap()
            .0;
        let customer_id = get_env_var("AUTH0_CLIENT_ID_LOCAL").unwrap();
        qdrant
            .upsert_points(points, &DbName::Log, &customer_id)
            .await
            .unwrap();

        let tool = Tool::LogRetrieval(LogRetrievalArgs::new(&testdata));
        let result = tool
            .request(&qdrant, &testdata.prompt, &customer_id)
            .await?;

        info!(result);
        assert!(result.contains("OOMKilled Exit Code 137"));
        Ok(())
    }
}
