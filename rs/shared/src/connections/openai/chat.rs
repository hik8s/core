use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionTool, CreateChatCompletionRequest, ResponseFormat,
};

use super::{
    openai::OpenAIConnection,
    tools::{LogRetrievalArgs, Tool},
};

impl OpenAIConnection {
    pub fn request_builder(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        model: &str,
        max_tokens: u32,
        num_choices: Option<u8>,
        response_format: Option<ResponseFormat>,
        tools: Option<Vec<ChatCompletionTool>>,
    ) -> CreateChatCompletionRequest {
        CreateChatCompletionRequest {
            model: model.to_string(),
            messages,
            max_tokens: Some(max_tokens),
            n: num_choices,
            response_format,
            tools,
            ..Default::default()
        }
    }
    pub fn chat_complete_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        model: &str,
        num_choices: u8,
    ) -> CreateChatCompletionRequest {
        // the args are not required for ChatCompletionTool
        let log_retrieval: ChatCompletionTool =
            Tool::LogRetrieval(LogRetrievalArgs::default()).into();
        let tools = vec![log_retrieval, Tool::ClusterOverview.into()];
        self.request_builder(messages, model, 1024, Some(num_choices), None, Some(tools))
    }
}

#[cfg(test)]
mod tests {
    use async_openai::error::OpenAIError;
    use rstest::rstest;
    use tokio::sync::mpsc;

    use crate::{
        connections::{
            openai::messages::{
                create_assistant_message, create_system_message, create_tool_message,
                create_user_message,
            },
            openai::tools::{collect_tool_call_chunks, Tool},
            OpenAIConnection,
        },
        constant::OPENAI_CHAT_MODEL_MINI,
        log_error,
        tracing::setup::setup_tracing,
    };

    #[tokio::test]
    #[rstest]
    #[case("logs".to_owned(), None, None, 1.0)]
    #[case("Get logs".to_owned(), None, None, 1.0)]
    #[case("Get logs!".to_owned(), None, None, 1.0)]
    #[case("Could you retrieve logs?".to_owned(), None, None, 1.0)]
    #[case("Could you retrieve logs for me?".to_owned(), None, None, 1.0)]
    #[case("Could you retrieve logs for the cluster for me?".to_owned(), None, None, 1.0)]
    #[case("Could you investigate the logs from logd in hik8s-system?".to_owned(), Some("logd".to_string()), Some("hik8s-system".to_string()), 1.0)]
    #[case("logd logs in hik8s-system?".to_owned(), Some("logd".to_string()), Some("hik8s-system".to_string()), 1.0)]
    #[case("logd logs in hik8s-system for me".to_owned(), Some("logd".to_string()), Some("hik8s-system".to_string()), 1.0)]
    async fn test_completion_log_retrieval(
        #[case] prompt: String,
        #[case] application: Option<String>,
        #[case] namespace: Option<String>,
        #[case] success_rate: f32,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let num_choices = 20;

        // base request
        let messages = vec![create_system_message(), create_user_message(&prompt)];
        let request =
            openai.chat_complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI, num_choices);
        let response = openai.create_completion(request).await?;

        // tool processing
        let mut success_counter: f32 = 0.0;
        for choice in response.choices {
            let tool_calls = choice.message.tool_calls.unwrap_or(Vec::new());
            assert!(tool_calls.len() <= 1, "Expected no more then on tool call.");
            for tool_call in tool_calls {
                let tool = Tool::try_from(tool_call.function).unwrap();
                assert!(matches!(tool, Tool::LogRetrieval(_)));
                if let Tool::LogRetrieval(args) = &tool {
                    assert_eq!(args.application, application);
                    assert_eq!(args.namespace, namespace);
                    assert!(!args.intention.is_empty());
                    success_counter += 1.0;
                }
            }
        }
        let res = success_counter / num_choices as f32;
        assert!(
            res >= success_rate,
            "Average of successful tool call not met: got {res} expected >= {success_rate} | case: {prompt}",
        );
        // let usage = response.usage.unwrap();
        // tracing::info!("completion tokens: {}", usage.completion_tokens);
        // tracing::info!("prompt tokens: {}", usage.prompt_tokens);
        // tracing::info!("total tokens: {}", usage.total_tokens);
        Ok(())
    }

    #[tokio::test]
    async fn test_stream_completion_tools() -> Result<(), OpenAIError> {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        // base request
        let prompt = "I have a problem with my application called logd in namespace hik8s-stag? Could you investigate the logs and also provide an overview of the cluster?";
        let mut messages = vec![create_system_message(), create_user_message(prompt)];
        let request = openai.chat_complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI, 1);
        let stream = openai.create_completion_stream(request).await?;

        let (_, tool_call_chunks) = openai
            .process_completion_stream(&tx, stream)
            .await
            .map_err(|e| log_error!(e))?;

        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }

        assert!(answer.is_empty());
        assert!(!tool_call_chunks.is_empty());

        // tool processing
        let tool_calls = collect_tool_call_chunks(tool_call_chunks);
        messages.push(create_assistant_message(
            "Assistent requested tool calls. Asses the tool calls and make another tool request to cluster overview",
            Some(tool_calls.clone()),
        ));
        for tool_call in tool_calls {
            let tool = Tool::try_from(tool_call.function).unwrap();
            messages.push(create_tool_message(&tool.test_request(), &tool_call.id));
        }

        // tool request
        let request = openai.chat_complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI, 1);
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let stream = openai.create_completion_stream(request).await?;
        let (_, tool_call_chunks) = openai
            .process_completion_stream(&tx, stream)
            .await
            .map_err(|e| log_error!(e))?;

        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        assert!(!answer.is_empty());
        assert!(tool_call_chunks.is_empty());
        Ok(())
    }
}
