use async_openai::{
    error::{ApiError, OpenAIError},
    types::{ResponseFormat, ResponseFormatJsonSchema},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json};
use shared::openai::{
    chat_request_args::{create_system_message, create_user_message},
    request_builder::create_chat_completion_request,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct UnderstandingOutput {
    pub application: Option<String>,
    pub namespace: Option<String>,
    pub keywords: Vec<String>,
    pub intention: Option<String>,
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
                    "description": "Keywords extracted that describe the user's intention.",
                },
                "intention": {
                    "type": "string",
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

async fn request_input_understandings(
    input: &str,
    num_choices: Option<u8>,
) -> Result<Vec<UnderstandingOutput>, OpenAIError> {
    // Create a new OpenAI client
    let client = Client::new();

    let messages = vec![create_system_message(), create_user_message(input)];

    // Construct the request
    let format = UnderstandingOutput::response_format();
    let request = create_chat_completion_request(messages, 100, num_choices, Some(format));

    // Send the request to the OpenAI API
    let response = client.chat().create(request).await?;
    let mut output = Vec::new();
    for choice in response.choices {
        if let Some(content) = &choice.message.content {
            let structured_output: UnderstandingOutput = from_str(content).unwrap();
            output.push(structured_output);
        }
    }
    if output.is_empty() {
        return Err(OpenAIError::ApiError(ApiError {
            message: "No structured output found".to_string(),
            r#type: None,
            param: None,
            code: None,
        }));
    }
    Ok(output)
}

pub async fn request_input_understanding(input: &str) -> Result<UnderstandingOutput, OpenAIError> {
    let output = request_input_understandings(input, Some(1)).await?;
    Ok(output.into_iter().next().unwrap())
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
