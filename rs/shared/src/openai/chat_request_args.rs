use async_openai::types::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent,
};

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
#[allow(deprecated)]
pub fn create_assistant_message(input: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
        content: Some(ChatCompletionRequestAssistantMessageContent::Text(
            input.to_string(),
        )),
        refusal: None,
        name: Some("assistant".to_string()),
        tool_calls: None,
        function_call: None,
    })
}
