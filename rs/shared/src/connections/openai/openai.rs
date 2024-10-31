use async_openai::{
    config::OpenAIConfig, error::OpenAIError, types::CreateChatCompletionStreamResponse, Client,
};
use futures_util::Stream;
use std::pin::Pin;
pub struct OpenAIConnection {
    pub client: Client<OpenAIConfig>,
}

impl OpenAIConnection {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    pub async fn chat_stream(
        &self,
        request: async_openai::types::CreateChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, OpenAIError>> + Send>>,
        OpenAIError,
    > {
        self.client.chat().create_stream(request).await
    }
}
