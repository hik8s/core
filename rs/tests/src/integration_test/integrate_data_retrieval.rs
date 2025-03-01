#[cfg(test)]
mod tests {
    use data_vectorizer::vectorize::vectorizer::vectorize_class_batch;
    use rstest::rstest;
    use shared::{
        connections::openai::{
            error::ToolRequestError,
            tools::{LogRetrievalArgs, Tool},
        },
        constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
        get_env_var, setup_tracing,
        testdata::{UserTest, UserTestData},
        types::tokenizer::Tokenizer,
        DbName, GreptimeConnection, QdrantConnection, RateLimiter,
    };
    use tracing::info;

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::PodKillOutOffMemory))]
    async fn test_retrieval_integration(
        #[case] testdata: UserTestData,
    ) -> Result<(), ToolRequestError> {
        // Prep
        setup_tracing(false);
        let greptime = GreptimeConnection::new().await.unwrap();
        let qdrant = QdrantConnection::new().await.unwrap();
        let tokenizer = Tokenizer::new().unwrap();
        let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);

        // Data ingestion
        let points = vectorize_class_batch(&[testdata.class.clone()], &tokenizer, &rate_limiter)
            .await
            .unwrap()
            .0;
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let db = DbName::Log.id(&customer_id);
        qdrant.upsert_points(points, &db).await.unwrap();

        let tool = Tool::LogRetrieval(LogRetrievalArgs::new(&testdata));
        let result = tool
            .request(&greptime, &qdrant, &testdata.prompt, &customer_id)
            .await?;

        info!(result);
        assert!(result.contains("OOMKilled Exit Code 137"));
        Ok(())
    }
}
