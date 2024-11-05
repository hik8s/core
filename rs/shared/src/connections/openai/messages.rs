use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestFunctionMessage,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent,
};
use tracing::warn;

use crate::constant::DEFAULT_SYSTEM_PROMPT;

pub fn create_system_message() -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
        content: ChatCompletionRequestSystemMessageContent::Text(DEFAULT_SYSTEM_PROMPT.to_string()),
        name: Some("System".to_string()),
    })
}
pub fn create_user_message(input: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text(input.to_string()),
        name: Some("User".to_string()),
    })
}
pub fn create_tool_message(input: &str, call_id: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
        content: ChatCompletionRequestToolMessageContent::Text(input.to_string()),
        tool_call_id: call_id.to_string(),
    })
}
pub fn create_function_message(input: &str, call_id: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Function(ChatCompletionRequestFunctionMessage {
        content: Some(input.to_string()),
        name: call_id.to_string(),
    })
}
#[allow(deprecated)]
pub fn create_assistant_message(
    input: &str,
    tool_calls: Option<Vec<ChatCompletionMessageToolCall>>,
) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
        content: Some(ChatCompletionRequestAssistantMessageContent::Text(
            input.to_string(),
        )),
        refusal: None,
        name: Some("assistant".to_string()),
        tool_calls,
        function_call: None,
    })
}

pub fn extract_last_user_text_message(messages: &[ChatCompletionRequestMessage]) -> String {
    messages
        .iter()
        .rev()
        .find_map(|msg| {
            if let ChatCompletionRequestMessage::User(user_msg) = msg {
                if let ChatCompletionRequestUserMessageContent::Text(content) = &user_msg.content {
                    Some(content.clone())
                } else {
                    warn!("User message with non text content. Using empty string");
                    None
                }
            } else {
                warn!("User message without content. Using empty string");
                None
            }
        })
        .unwrap_or_default()
}
