use async_openai::types::{
    ChatCompletionMessageToolCallChunk, ChatCompletionRequestMessage, ChatCompletionTool,
    CreateChatCompletionRequest, CreateChatCompletionResponse, FinishReason, ResponseFormat,
};
use async_openai::{
    config::OpenAIConfig, error::OpenAIError, types::CreateChatCompletionStreamResponse, Client,
};
use futures_util::Stream;
use futures_util::StreamExt;
use std::pin::Pin;
use tokio::sync::mpsc;

use super::tools::{LogRetrievalArgs, Tool};

pub struct OpenAIConnection {
    pub client: Client<OpenAIConfig>,
}

impl OpenAIConnection {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    // COMPETION REQUEST
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
    // CHAT COMPLETION
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
