use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestDeveloperMessageContent,
    ChatCompletionRequestFunctionMessage, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent,
    ChatCompletionRequestToolMessage, ChatCompletionRequestToolMessageContent,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
};
use tracing::warn;

use crate::constant::DEFAULT_SYSTEM_PROMPT;

pub fn create_system_message() -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
        content: ChatCompletionRequestSystemMessageContent::Text(DEFAULT_SYSTEM_PROMPT.to_string()),
        name: Some("System".to_string()),
    })
}
pub fn create_simple_system_message() -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
        content: ChatCompletionRequestSystemMessageContent::Text(
            "You are a helpful assistant".to_string(),
        ),
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
        audio: None,
    })
}

pub fn extract_last_user_text_message(messages: &[ChatCompletionRequestMessage]) -> String {
    messages
        .iter()
        .rev()
        .find_map(|msg| {
            if let ChatCompletionRequestMessage::User(_) = msg {
                extract_message_content(msg)
            } else {
                warn!("User message without content. Using empty string");
                None
            }
        })
        .unwrap_or_default()
}

pub fn extract_message_content(msg: &ChatCompletionRequestMessage) -> Option<String> {
    match msg {
        ChatCompletionRequestMessage::User(user_msg) => {
            if let ChatCompletionRequestUserMessageContent::Text(content) = &user_msg.content {
                Some(content.clone())
            } else {
                warn!("User message with non text content");
                None
            }
        }
        ChatCompletionRequestMessage::System(system_msg) => {
            if let ChatCompletionRequestSystemMessageContent::Text(content) = &system_msg.content {
                Some(content.clone())
            } else {
                warn!("System message with non text content");
                None
            }
        }
        ChatCompletionRequestMessage::Assistant(assistant_msg) => {
            assistant_msg.content.as_ref().and_then(|content| {
                if let ChatCompletionRequestAssistantMessageContent::Text(text) = content {
                    Some(text.clone())
                } else {
                    warn!("Assistant message with non text content");
                    None
                }
            })
        }
        ChatCompletionRequestMessage::Tool(tool_msg) => {
            if let ChatCompletionRequestToolMessageContent::Text(content) = &tool_msg.content {
                Some(content.clone())
            } else {
                warn!("Tool message with non text content");
                None
            }
        }
        ChatCompletionRequestMessage::Function(function_msg) => function_msg.content.clone(),
        ChatCompletionRequestMessage::Developer(dev_msg) => {
            if let ChatCompletionRequestDeveloperMessageContent::Text(text) = &dev_msg.content {
                Some(text.clone())
            } else {
                warn!("Developer message with non text content");
                None
            }
        }
    }
}
