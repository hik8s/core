#[cfg(test)]
mod tests {
    use async_openai::{error::OpenAIError, types::ChatCompletionRequestMessage};
    use chat_backend::chat::process::{process_user_message, RequestOptions};
    use data_vectorizer::vectorize::vectorize_classes;
    use prompt_engine::{
        prompt::test_prompt::{get_scenario_data, ClusterTestScenario},
        server::initialize_prompt_engine,
    };
    use rstest::rstest;
    use tokio::sync::mpsc;

    use shared::{
        connections::{
            openai::messages::extract_message_content,
            prompt_engine::connect::PromptEngineConnection, qdrant::connect::QdrantConnection,
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
        tokio::spawn(async move {
            let rocket = initialize_prompt_engine().await.unwrap();
            rocket.launch().await.unwrap()
        });
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
        let prompt_engine = PromptEngineConnection::new().unwrap();
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();

        let prompt = format!("I have a problem with my application called {application} in namespace {namespace}? Could you investigate the logs and also provide an overview of the cluster?");
        let request_option = RequestOptions::new(&prompt, &client_id);
        let mut messages: Vec<ChatCompletionRequestMessage> = request_option.clone().into();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        process_user_message(&prompt_engine, &mut messages, &tx, request_option)
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
        assert!(tool_content.contains(&format!("{representation}")));
        assert!(!answer.is_empty());
        // tracing::info!("Messages: {:#?}", messages);
        // tracing::info!("Answer: {:#?}", answer);
        assert!(answer.contains(&representation)); // this could fail
        Ok(())
    }
}
