use async_openai::error::ApiError;
use async_openai::types::{
    ChatCompletionMessageToolCallChunk, ChatCompletionRequestMessage, ChatCompletionTool,
    CreateChatCompletionRequest, CreateChatCompletionResponse, FinishReason, ResponseFormat,
    ResponseFormatJsonSchema,
};
use async_openai::{
    config::OpenAIConfig, error::OpenAIError, types::CreateChatCompletionStreamResponse, Client,
};
use futures_util::Stream;
use futures_util::StreamExt;
use serde_json::json;
use std::pin::Pin;
use tokio::sync::mpsc;

use crate::constant::OPENAI_CHAT_MODEL_MINI;

use super::messages::create_simple_system_message;
use super::tools::list_all_tools;

pub struct OpenAIConnection {
    pub client: Client<OpenAIConfig>,
}

impl Default for OpenAIConnection {
    // https://rust-lang.github.io/rust-clippy/master/index.html#new_without_default
    fn default() -> Self {
        Self::new()
    }
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
        ////////////////////////////////////////////////////////////////////////////////////////////////
        //////////////// TODO: inject current state / topology of the cluster here  ///////////////////
        ////////////////////////////////////////////////////////////////////////////////////////////////

        let tools = list_all_tools();
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
        tx: &mpsc::UnboundedSender<CreateChatCompletionStreamResponse>,
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
        let mut client_disconnected = false;

        while let Some(result) = stream.next().await {
            let response = result?;
            let choice = response
                .choices
                .first()
                .ok_or(OpenAIError::ApiError(create_no_choice_error()))?;

            if let Some(ref tool_call_chunk) = choice.delta.tool_calls {
                tool_call_chunks.extend_from_slice(tool_call_chunk);
            }
            let response_clone = response.clone();
            if let Some(ref _delta) = choice.delta.content {
                // Only try sending if the client is still connected
                // if let Some(ref _delta) = choice.delta.content {
                //     tx.send(response_clone).unwrap();
                // }
                if !client_disconnected && tx.send(response_clone).is_err() {
                    tracing::warn!("Client disconnected. Will still finish reading stream.");
                    client_disconnected = true;
                }
            }
            finish_reason = choice.finish_reason;
        }
        Ok((finish_reason, tool_call_chunks))
    }

    pub async fn ask_atomic_question(&self, question: &str) -> Result<bool, OpenAIError> {
        let schema = json!({
            "type": "object",
            "properties": {
                "answer": {
                    "type": "boolean",
                    "description": "Does the question apply true for yes, false for no.",
                },
            },
            "additionalProperties": false,
            "required": ["answer"]
        });
        let response_format = self.response_format(question, schema);
        let messages = vec![create_simple_system_message()];

        let request = self.request_builder(
            messages,
            OPENAI_CHAT_MODEL_MINI,
            100,
            Some(1),
            Some(response_format),
            None,
        );

        let mut response = self.create_completion(request).await?;
        let json = response.choices.pop().unwrap().message.content.unwrap();
        let json_parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        Ok(json_parsed["answer"].as_bool().unwrap())
    }
    pub fn response_format(&self, description: &str, schema: serde_json::Value) -> ResponseFormat {
        ResponseFormat::JsonSchema {
            json_schema: ResponseFormatJsonSchema {
                name: "question".to_string(),
                description: Some(description.to_owned()),
                schema: Some(schema),
                strict: Some(true),
            },
        }
    }
}

fn create_no_choice_error() -> ApiError {
    ApiError {
        message: "No choices available in response".to_string(),
        r#type: Some("invalid_response".to_string()),
        param: Some("choices".to_string()),
        code: Some("no_choices".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::setup_tracing;

    use super::*;

    #[tokio::test]
    async fn test_ask_atomic_question() -> Result<(), OpenAIError> {
        setup_tracing(false);
        let openai = OpenAIConnection::new();

        // Test with a simple true/false question
        let question = "Is Rust a programming language?";
        let result = openai.ask_atomic_question(question).await?;

        assert!(result, "Expected true for a factual question");

        Ok(())
    }
}
