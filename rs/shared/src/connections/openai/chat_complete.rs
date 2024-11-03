use crate::connections::openai::tools::{collect_tool_call_chunks, Tool};
use crate::connections::prompt_engine::connect::PromptEngineConnection;
use crate::connections::OpenAIConnection;
use crate::log_error;

use async_openai::error::OpenAIError;
use async_openai::types::{ChatCompletionRequestMessage, FinishReason};

use tokio::sync::mpsc;

use super::messages::{create_assistant_message, create_tool_message};

pub async fn request_completion(
    prompt_engine: &PromptEngineConnection,
    openai: &OpenAIConnection,
    messages: &mut Vec<ChatCompletionRequestMessage>,
    last_user_content: String,
    customer_id: String,
    model: &str,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), OpenAIError> {
    loop {
        let request = openai.chat_complete_request(messages.clone(), model);
        let stream = openai
            .create_completion_stream(request)
            .await
            .map_err(|e| log_error!(e))?;
        let (finish_reason, tool_call_chunks) = openai
            .process_completion_stream(tx, stream)
            .await
            .map_err(|e| log_error!(e))?;

        if finish_reason == Some(FinishReason::Stop) {
            break;
        }

        let tool_calls = collect_tool_call_chunks(tool_call_chunks);
        if tool_calls.is_empty() {
            break;
        }

        messages.push(create_assistant_message(
            "Assistant requested tool calls.",
            Some(tool_calls.clone()),
        ));
        for tool_call in tool_calls {
            let tool = Tool::try_from(&tool_call.function.name).unwrap();
            let tool_output = tool
                .request(prompt_engine, &last_user_content, &customer_id)
                .await
                .unwrap();
            let tool_message = create_tool_message(&tool_output, &tool_call.id);
            messages.push(tool_message);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use async_openai::{error::OpenAIError, types::ChatCompletionRequestMessage};
    use tokio::sync::mpsc;

    use super::request_completion;
    use crate::{
        connections::{
            openai::messages::{create_system_message, create_user_message},
            prompt_engine::connect::PromptEngineConnection,
            OpenAIConnection,
        },
        constant::OPENAI_CHAT_MODEL_MINI,
        get_env_var,
        tracing::setup::setup_tracing,
    };

    #[tokio::test]
    async fn test_completion_request() -> Result<(), OpenAIError> {
        setup_tracing(false);
        let prompt_engine = PromptEngineConnection::new().unwrap();
        let openai = OpenAIConnection::new();
        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();

        let prompt = "I have a problem with my application called logd in namespace hik8s-stag? Could you investigate the logs and also provide an overview of the cluster?";
        let mut messages = vec![create_system_message(), create_user_message(prompt)];
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        request_completion(
            &prompt_engine,
            &openai,
            &mut messages,
            prompt.to_string(),
            client_id,
            OPENAI_CHAT_MODEL_MINI,
            &tx,
        )
        .await
        .unwrap();

        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }

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
