#[cfg(test)]
mod tests {
    use std::{iter::zip, time::Instant};

    use async_openai::{error::OpenAIError, types::ChatCompletionRequestMessage};
    use bm25::{Language, SearchEngineBuilder};
    use chat_backend::chat::process::{process_user_message, RequestOptions};
    use data_vectorizer::vectorize_class_batch;
    use rstest::rstest;
    use shared::{
        connections::dbname::DbName,
        testdata::{UserTest, UserTestData},
    };
    use tokio::sync::mpsc;

    use shared::{
        connections::{
            openai::messages::extract_message_content, qdrant::connect::QdrantConnection,
            OpenAIConnection,
        },
        constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
        get_env_var,
        tracing::setup::setup_tracing,
        types::tokenizer::Tokenizer,
        utils::ratelimit::RateLimiter,
    };

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::PodKillOutOffMemory))]
    async fn test_process_user_message(#[case] testdata: UserTestData) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();
        let tokenizer = Tokenizer::new().unwrap();
        let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);

        // Data ingestion
        let points =
            vectorize_class_batch(&vec![testdata.class.clone()], &tokenizer, &rate_limiter)
                .await
                .unwrap()
                .0;
        let customer_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
        qdrant
            .upsert_points(points, &DbName::Log, &customer_id)
            .await
            .unwrap();

        // Prompt processing
        let request_option = RequestOptions::new(&testdata.prompt, &customer_id);
        let mut messages: Vec<ChatCompletionRequestMessage> = request_option.clone().into();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        process_user_message(&qdrant, &mut messages, &tx, request_option)
            .await
            .unwrap();

        // Answer evaluation
        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        assert!(!answer.is_empty());
        assert_eq!(messages.len(), 5);
        for (message, expected_message) in zip(&messages, &testdata.messages) {
            assert!(std::mem::discriminant(message) == std::mem::discriminant(expected_message));
        }

        // log-retrieval evaluation
        let tool_content = extract_message_content(&messages[3]).unwrap_or_default();
        assert!(!answer.is_empty());

        // LLM check
        let start = Instant::now();
        let openai = OpenAIConnection::new();
        let match_str = "OOM killed exit code 137";
        let question = format!(
            "Does the following answer definitely conclude that the pod was {match_str}? {answer}"
        );
        let evaluation = openai.ask_atomic_question(&question).await.unwrap();
        tracing::info!("Atomic question took: {:?}", start.elapsed());

        // bm25 match
        let start = Instant::now();
        let search_engine =
            SearchEngineBuilder::<u32>::with_corpus(Language::English, UserTest::corpus()).build();
        let search_results = search_engine.search(&answer, 3);

        for result in search_results {
            tracing::info!(
                "Document ID: {}, Score: {}",
                result.document.id,
                result.score
            );
            tracing::info!("Document Content: {}", result.document.contents);
            tracing::info!("---");
        }
        assert!(evaluation, "evaluation: {evaluation} question: {question}");
        assert!(tool_content.contains(&format!("{}", &testdata.class.to_string())));
        tracing::info!("Search and evaluation took: {:?}", start.elapsed());
        Ok(())
    }
}
