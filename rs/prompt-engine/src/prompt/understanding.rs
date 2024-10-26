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
    use crate::prompt::test_prompt::{get_expected_output, UserInputTestCase};

    use super::request_input_understandings;
    use rstest::rstest;
    use shared::tracing::setup::setup_tracing;
    use tracing::debug;

    #[tokio::test]
    #[rstest]
    #[case(UserInputTestCase::WhatIsGoingOn)]
    #[case(UserInputTestCase::WhatIsGoingOnWithApp)]
    #[case(UserInputTestCase::WhatIsGoingOnWithNamespace)]
    #[case(UserInputTestCase::WhatIsGoingOnWithAppWithNamespace)]
    async fn test_send_request_to_openai(#[case] test_case: UserInputTestCase) {
        // Initialize tracing subscriber for logging
        setup_tracing(false);
        let num_choices = 10;
        // let user_input = "What is going on I see lots of errors with logd in hik8s-stag?";
        let user_input = test_case.to_string();
        let expected_output = get_expected_output(test_case);
        let structured_outputs = request_input_understandings(&user_input, Some(num_choices))
            .await
            .unwrap();

        let mut res_app = Vec::new();
        let mut res_namespace = Vec::new();
        for output in structured_outputs {
            res_app.push((output.application == expected_output.application) as u32);
            res_namespace.push((output.namespace == expected_output.namespace) as u32);
        }
        let mean_res_app: f32 = res_app.iter().sum::<u32>() as f32 / res_app.len() as f32;
        let mean_res_namespace: f32 =
            res_namespace.iter().sum::<u32>() as f32 / res_namespace.len() as f32;

        debug!(
            "Mean res (app|ns): ({}|{})",
            mean_res_app, mean_res_namespace
        );
        assert!(
            mean_res_app >= 0.8,
            "Mean of successful extracted app ({:?}) not met: violated {} >= 0.8",
            expected_output.application,
            mean_res_app
        );
        assert!(
            mean_res_namespace >= 0.8,
            "Mean of successful extracted app ({:?}) not met: violated {} >= 0.8",
            expected_output.application,
            mean_res_namespace
        );
    }
}
