#[cfg(test)]
mod tests {
    use std::time::Instant;

    use async_openai::{error::OpenAIError, types::ChatCompletionRequestMessage};
    use bm25::{Language, SearchEngineBuilder};
    use chat_backend::chat::process::{process_user_message, RequestOptions};
    use data_vectorizer::vectorize::vectorize_classes;
    use prompt_engine::prompt::test_prompt::{get_scenario_data, ClusterTestScenario};
    use rstest::rstest;
    use tokio::sync::mpsc;

    use shared::{
        connections::{
            openai::messages::extract_message_content, qdrant::connect::QdrantConnection,
            OpenAIConnection,
        },
        constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
        get_db_name, get_env_var,
        tracing::setup::setup_tracing,
        types::tokenizer::Tokenizer,
        utils::ratelimit::RateLimiter,
    };

    #[tokio::test]
    #[rstest]
    #[case(ClusterTestScenario::PodKillOutOffMemory)]
    async fn test_process_user_message(
        #[case] test_scenario: ClusterTestScenario,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();
        let tokenizer = Tokenizer::new().unwrap();
        let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);

        // Data ingestion
        let scenario_data = get_scenario_data(&test_scenario);
        let application = scenario_data.vectorized_classes[0].key.to_owned();
        let namespace = scenario_data.vectorized_classes[0].namespace.to_owned();
        let representation = scenario_data.vectorized_classes[0].to_string();
        let points =
            vectorize_classes(&scenario_data.vectorized_classes, &tokenizer, &rate_limiter)
                .await
                .unwrap()
                .0;
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
        let db_name = get_db_name(&client_id);
        qdrant.upsert_points(points, &db_name).await.unwrap();

        // Prompt processing
        let qdrant = QdrantConnection::new().await.unwrap();
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();

        let prompt = format!("I have a problem with my application called {application} in namespace {namespace}? Could you investigate the logs and also provide an overview of the cluster?");
        let request_option = RequestOptions::new(&prompt, &client_id);
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
        assert!(matches!(
            messages[0],
            ChatCompletionRequestMessage::System(_)
        ));
        assert!(matches!(messages[1], ChatCompletionRequestMessage::User(_)));
        assert!(matches!(
            messages[2],
            ChatCompletionRequestMessage::Assistant(_)
        ));
        assert!(matches!(messages[3], ChatCompletionRequestMessage::Tool(_)));
        assert!(matches!(messages[4], ChatCompletionRequestMessage::Tool(_)));

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
        tracing::info!("Search and evaluation took: {:?}", start.elapsed());
        let control_string1 = "I've retrieved the logs for the application in the namespace, but it appears that the logs are empty.".to_owned();
        let control_string2 = format!("I've retrieved the logs for the application in the namespace, but it appears that there is not problem found.");
        let target_string = format!("I've retrieved the logs for the application in the namespace, and it appears that it was {match_str}");
        let corpus = vec![target_string, control_string1, control_string2];
        let search_engine =
            SearchEngineBuilder::<u32>::with_corpus(Language::English, corpus).build();
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
        assert!(tool_content.contains(&format!("{representation}")));
        tracing::info!("Search and evaluation took: {:?}", start.elapsed());
        Ok(())
    }
}
