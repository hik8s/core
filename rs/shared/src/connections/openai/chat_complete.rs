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
