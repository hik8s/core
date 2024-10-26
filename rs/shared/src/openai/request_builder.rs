use async_openai::types::{
    ChatCompletionRequestMessage, CreateChatCompletionRequest, ResponseFormat,
};

use crate::constant::OPENAI_CHAT_MODEL_MINI;

pub fn create_chat_completion_request(
    messages: Vec<ChatCompletionRequestMessage>,
    max_tokens: u32,
    num_choices: Option<u8>,
    response_format: Option<ResponseFormat>,
) -> CreateChatCompletionRequest {
    CreateChatCompletionRequest {
        model: OPENAI_CHAT_MODEL_MINI.to_string(),
        messages,
        max_tokens: Some(max_tokens),
        n: num_choices,
        response_format,
        ..Default::default()
    }
}
