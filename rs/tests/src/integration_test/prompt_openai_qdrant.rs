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
            openai::messages::{create_system_message, create_user_message},
            prompt_engine::connect::PromptEngineConnection,
            qdrant::connect::QdrantConnection,
            OpenAIConnection,
        },
        constant::{OPENAI_CHAT_MODEL_MINI, OPENAI_EMBEDDING_TOKEN_LIMIT},
        get_db_name, get_env_var,
        tracing::setup::setup_tracing,
        types::tokenizer::Tokenizer,
        utils::ratelimit::RateLimiter,
    };
    use tracing::info;

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

        // Prompt processing
        let prompt_engine = PromptEngineConnection::new().unwrap();
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();

        let prompt = "I have a problem with my application called logd in namespace hik8s-stag? Could you investigate the logs and also provide an overview of the cluster?";
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
        assert!(!answer.is_empty());
        Ok(())
    }
}
