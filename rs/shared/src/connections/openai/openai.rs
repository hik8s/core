use async_openai::types::{
    ChatCompletionMessageToolCallChunk, CreateChatCompletionResponse, FinishReason,
};
use async_openai::{
    config::OpenAIConfig, error::OpenAIError, types::CreateChatCompletionStreamResponse, Client,
};
use futures_util::Stream;
use futures_util::StreamExt;
use std::pin::Pin;
use tokio::sync::mpsc;

pub struct OpenAIConnection {
    pub client: Client<OpenAIConfig>,
}

impl OpenAIConnection {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn create_completion(
        &self,
        request: async_openai::types::CreateChatCompletionRequest,
    ) -> Result<CreateChatCompletionResponse, OpenAIError> {
        self.client.chat().create(request).await
    }
    pub async fn create_completion_stream(
        &self,
        request: async_openai::types::CreateChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, OpenAIError>> + Send>>,
        OpenAIError,
    > {
        self.client.chat().create_stream(request).await
    }
    pub async fn process_completion_stream(
        &self,
        tx: &mpsc::UnboundedSender<String>,
        mut stream: Pin<
            Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, OpenAIError>> + Send>,
        >,
    ) -> Result<
        (
            Option<FinishReason>,
            Vec<ChatCompletionMessageToolCallChunk>,
        ),
        OpenAIError,
    > {
        let mut finish_reason = Option::<FinishReason>::None;
        let mut tool_call_chunks = Vec::<ChatCompletionMessageToolCallChunk>::new();
        while let Some(result) = stream.next().await {
            let response = result?;
            let choice = response.choices.first().unwrap();
            if let Some(ref tool_call_chunk) = choice.delta.tool_calls {
                tool_call_chunks.extend_from_slice(&tool_call_chunk);
            }
            if let Some(ref delta) = choice.delta.content {
                tx.send(delta.to_owned()).unwrap();
            }
            finish_reason = choice.finish_reason.clone();
        }
        Ok((finish_reason, tool_call_chunks))
    }
}
