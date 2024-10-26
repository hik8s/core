use async_openai::{
    error::OpenAIError,
    types::{
        CreateChatCompletionRequestArgs, CreateChatCompletionResponse, ResponseFormat,
        ResponseFormatJsonSchema,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    constant::OPENAI_CHAT_MODEL,
    openai::chat_request_args::{create_system_message, create_user_message},
};

#[derive(Debug, Serialize, Deserialize)]
struct UnderstandingOutput {
    application: Option<String>,
    namespace: Option<String>,
    keywords: Vec<String>,
    intention: Option<String>,
}
impl UnderstandingOutput {
    fn get_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "application": {
                    "type": ["string", "null"],
                    "description": "Name of the application, service, or container. If no indication is provided do not return any value",
                },
                "namespace": {
                    "type": ["string", "null"],
                    "description": "Name of the namespace"
                },
                "keywords": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                },
                "intention": {
                    "type": ["string", "null"],
                    "description": "The intention of the user. What does the user want to achieve?"
                }
            },
            "additionalProperties": false,
            "required": ["keywords", "application", "namespace", "intention"]
        })
    }
    pub fn response_format() -> ResponseFormat {
        ResponseFormat::JsonSchema {
            json_schema: ResponseFormatJsonSchema {
                name: "understanding_output".to_string(),
                description: Some(
                    "Structured output containing keywords, application, namespace, and intention"
                        .to_string(),
                ),
                schema: Some(Self::get_schema()),
                strict: Some(true),
            },
        }
    }
}

async fn send_request_to_openai(input: &str) -> Result<CreateChatCompletionResponse, OpenAIError> {
    // Create a new OpenAI client
    let client = Client::new();

    let messages = vec![create_system_message(), create_user_message(input)];

    // Construct the request
    let request = CreateChatCompletionRequestArgs::default()
        .model(OPENAI_CHAT_MODEL)
        .messages(messages)
        .max_tokens(100_u32)
        .n(1)
        .response_format(UnderstandingOutput::response_format())
        .build()
        .unwrap();

    // Send the request to the OpenAI API
    let response = client.chat().create(request).await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{send_request_to_openai, UnderstandingOutput};
    use serde_json::from_str;
    use shared::tracing::setup::setup_tracing;

    #[tokio::test]
    async fn test_send_request_to_openai() {
        // Initialize tracing subscriber for logging
        setup_tracing(false);
        // let user_input = "What is going on I see lots of errors with logd in hik8s-stag?";
        let user_input = "What is going on?";
        for choice in send_request_to_openai(&user_input).await.unwrap().choices {
            if let Some(content) = &choice.message.content {
                let structured_output: UnderstandingOutput =
                    from_str(content).expect("Failed to parse JSON");
                tracing::info!("Parsed structured output: {:#?}", structured_output);
            } else {
                panic!("The response should not be empty");
            }
        }
    }
}
