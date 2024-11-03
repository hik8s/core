#[cfg(test)]
mod tests {
    use async_openai::error::OpenAIError;
    use chat_backend::chat::process::{process_user_message, Message, RequestOptions};
    use tokio::sync::mpsc;

    use shared::{
        connections::prompt_engine::connect::PromptEngineConnection,
        constant::OPENAI_CHAT_MODEL_MINI, get_env_var, tracing::setup::setup_tracing,
    };

    #[tokio::test]
    async fn test_process_user_message() -> Result<(), OpenAIError> {
        setup_tracing(false);
        let prompt_engine = PromptEngineConnection::new().unwrap();
        // let openai = OpenAIConnection::new();
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();

        let prompt = "I have a problem with my application called logd in namespace hik8s-stag? Could you investigate the logs and also provide an overview of the cluster?";
        let request_option = RequestOptions {
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "not used".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            model: OPENAI_CHAT_MODEL_MINI.to_string(),
            client_id,
            temperature: None,
            top_p: None,
        };
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        process_user_message(&prompt_engine, request_option, &tx)
            .await
            .unwrap();

        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        assert!(!answer.is_empty());
        Ok(())
    }
}
