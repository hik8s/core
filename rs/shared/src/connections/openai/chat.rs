use async_openai::{
    error::OpenAIError,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestMessage, CreateChatCompletionRequest,
        CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
    },
};

use super::{openai::OpenAIConnection, tools::Tool};

impl OpenAIConnection {
    pub fn complete_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        model: &str,
    ) -> Result<CreateChatCompletionRequest, OpenAIError> {
        let tools = vec![Tool::LogRetrieval.into(), Tool::ClusterOverview.into()];

        CreateChatCompletionRequestArgs::default()
            .model(model)
            .max_tokens(1024u16)
            .messages(messages)
            .tools(tools)
            .build()
    }
}

pub fn get_tool_calls(
    response: &CreateChatCompletionResponse,
) -> Option<Vec<ChatCompletionMessageToolCall>> {
    if let Some(choice) = response.choices.first() {
        return choice.message.tool_calls.clone();
    }
    None
}

#[cfg(test)]
mod tests {
    use async_openai::types::ChatCompletionMessageToolCallChunk;
    use tracing::info;

    use crate::{
        connections::{
            openai::tools::{collect_tool_call_chunks, Tool},
            prompt_engine::connect::PromptEngineConnection,
            OpenAIConnection,
        },
        constant::OPENAI_CHAT_MODEL_MINI,
        get_env_var, log_error, log_error_continue,
        openai::chat_request_args::{
            create_assistant_message, create_system_message, create_tool_message,
            create_user_message,
        },
        tracing::setup::setup_tracing,
    };

    use super::get_tool_calls;

    #[tokio::test]
    async fn test_completion_tools() {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let prompt_engine = PromptEngineConnection::new().unwrap();
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();

        // base request
        let prompt = "I have a problem with my application called logd in namespace hik8s-stag? Could you investigate the logs and also provide an overview of the cluster?";
        let mut messages = vec![create_system_message(), create_user_message(prompt)];
        let request = openai
            .complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI)
            .unwrap();
        let response = openai.client.chat().create(request).await.unwrap();

        // tool processing
        let tool_calls = get_tool_calls(&response).unwrap();
        messages.push(create_assistant_message(
            "this should be empty",
            Some(tool_calls.clone()),
        ));
        for tool_call in tool_calls {
            let tool = Tool::try_from(&tool_call.function.name).unwrap();
            messages.push(create_tool_message(
                &tool
                    .request(&prompt_engine, &prompt, client_id.as_str())
                    .await
                    .unwrap(),
                &tool_call.id,
            ));
        }

        // tool request
        let request = openai
            .complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI)
            .unwrap();
        let response = openai.client.chat().create(request).await.unwrap();
        info!("{:#?}", response);
    }

    use futures_util::StreamExt;
    #[tokio::test]
    async fn test_completion_tools_stream() {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let prompt_engine = PromptEngineConnection::new().unwrap();
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();

        // base request
        let prompt = "I have a problem with my application called logd in namespace hik8s-stag? Could you investigate the logs and also provide an overview of the cluster?";
        let mut messages = vec![create_system_message(), create_user_message(prompt)];
        let request = openai
            .complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI)
            .unwrap();
        let mut stream = openai.client.chat().create_stream(request).await.unwrap();
        let mut tool_call_chunks = Vec::<ChatCompletionMessageToolCallChunk>::new();
        let mut answer = String::new();
        while let Some(result) = stream.next().await {
            let response = log_error_continue!(result);
            let choice = response.choices.first().unwrap();
            // info!("{:#?}", choice.delta.content);
            if let Some(ref tool_call_chunk) = choice.delta.tool_calls {
                tool_call_chunks.extend_from_slice(&tool_call_chunk);
            }
            if let Some(ref content) = choice.delta.content {
                answer.push_str(content);
            }
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
            let tool = Tool::try_from(&tool_call.function.name).unwrap();
            messages.push(create_tool_message(
                &tool
                    .request(&prompt_engine, &prompt, client_id.as_str())
                    .await
                    .unwrap(),
                &tool_call.id,
            ));
        }

        // tool request
        let request = openai
            .complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI)
            .unwrap();
        let mut stream = openai.client.chat().create_stream(request).await.unwrap();
        let mut answer = String::new();
        let mut tool_call_chunks_v2 = Vec::<ChatCompletionMessageToolCallChunk>::new();
        while let Some(result) = stream.next().await {
            let response = log_error_continue!(result);
            let choice = response.choices.first().unwrap();
            if let Some(ref tool_call_chunk) = choice.delta.tool_calls {
                tool_call_chunks_v2.extend_from_slice(&tool_call_chunk);
            }
            if let Some(ref content) = choice.delta.content {
                answer.push_str(content);
            }
        }
        info!("{:#?}", answer);
        info!("{:#?}", tool_call_chunks_v2);
        assert!(!answer.is_empty());
        assert!(tool_call_chunks_v2.is_empty());
    }
}
