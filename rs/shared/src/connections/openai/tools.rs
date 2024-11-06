use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk, ChatCompletionTool,
    ChatCompletionToolType, FunctionCall, FunctionObject,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use std::fmt;

use crate::connections::prompt_engine::connect::{
    AugmentationRequest, PromptEngineConnection, PromptEngineError,
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LogRetrievalArgs {
    pub namespace: Option<String>,
    pub application: Option<String>,
    pub intention: String,
}
impl TryFrom<String> for LogRetrievalArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}
#[derive(Debug)]
pub enum Tool {
    ClusterOverview,
    LogRetrieval(LogRetrievalArgs),
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tool_name = match self {
            Tool::ClusterOverview => "cluster-overview",
            Tool::LogRetrieval(_) => "log-retrieval",
        };
        write!(f, "{}", tool_name)
    }
}
impl TryFrom<FunctionCall> for Tool {
    type Error = String;

    fn try_from(call: FunctionCall) -> Result<Self, Self::Error> {
        let FunctionCall { name, arguments } = call;
        match name.as_str() {
            "cluster-overview" => Ok(Tool::ClusterOverview),
            "log-retrieval" => Ok(Tool::LogRetrieval(arguments.try_into().unwrap())),
            _ => Err(format!("Could not parse tool name: {}", name)),
        }
    }
}
impl Tool {
    pub fn get_function(&self) -> FunctionObject {
        match self {
            Tool::ClusterOverview => FunctionObject {
                name: self.to_string(),
                description: Some(
                    "Retrieve an overview of the cluster and its applications".to_string(),
                ),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "resource": {
                            "type": ["string", "null"],
                            "description": "The type of resource to retrieve. For example, 'pods', 'services', 'deployments', etc. None for all resources.",
                        },
                        "namespace": {
                            "type": ["string", "null"],
                            "description": "Name of the namespace"
                        },
                    },
                    "additionalProperties": false,
                    "required": ["resource", "namespace"]
                })),
                strict: Some(true),
            },
            Tool::LogRetrieval(_) => FunctionObject {
                name: self.to_string(),
                description: Some("Retrieve logs from the kubernetes cluster".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "application": {
                            "type": ["string", "null"],
                            "description": "Name of the application, service, or container",
                        },
                        "namespace": {
                            "type": ["string", "null"],
                            "description": "Name of the namespace"
                        },
                        "intention": {
                            "type": "string",
                            "description": "The users intention. What does the user want to achieve with their question?"
                        }
                    },
                    "additionalProperties": false,
                    "required": ["application", "namespace", "intention"]
                })),
                strict: Some(true),
            },
        }
    }
    pub async fn request(
        &self,
        prompt_engine: &PromptEngineConnection,
        user_message: &str,
        customer_id: &str,
    ) -> Result<String, PromptEngineError> {
        match self {
            Tool::ClusterOverview => Ok("We got a bunch of pods here and then theres this huge pile of containers in this corner of the cluster".to_string()),
            Tool::LogRetrieval(_) => {
                let request = AugmentationRequest::new(&user_message, &customer_id);
                prompt_engine.request_augmentation(request).await
            },
        }
    }
    pub fn test_request(&self) -> String {
        match self {
            Tool::ClusterOverview => "We got a bunch of pods here and then theres this huge pile of containers in this corner of the cluster".to_string(),
            Tool::LogRetrieval(_) => "OOMKilled exit code 137".to_owned(),
        }
    }
}

impl Into<ChatCompletionTool> for Tool {
    fn into(self) -> ChatCompletionTool {
        ChatCompletionTool {
            r#type: ChatCompletionToolType::Function,
            function: self.get_function(),
        }
    }
}

pub fn collect_tool_call_chunks(
    tool_call_chunks: Vec<ChatCompletionMessageToolCallChunk>,
) -> Vec<ChatCompletionMessageToolCall> {
    let mut tool_calls = Vec::<ChatCompletionMessageToolCall>::new();
    if !tool_call_chunks.is_empty() {
        for chunk in tool_call_chunks {
            if let Some(call_id) = chunk.id {
                tool_calls.push(ChatCompletionMessageToolCall {
                    id: call_id,
                    r#type: ChatCompletionToolType::Function,
                    function: FunctionCall {
                        name: "".to_string(),
                        arguments: "".to_string(),
                    },
                });
            }
            if let Some(funcion_call_stream) = chunk.function {
                if let Some(name) = funcion_call_stream.name {
                    tool_calls.last_mut().unwrap().function.name.push_str(&name);
                }
                if let Some(arguments) = funcion_call_stream.arguments {
                    tool_calls
                        .last_mut()
                        .unwrap()
                        .function
                        .arguments
                        .push_str(&arguments);
                }
            }
        }
    }
    tool_calls
}

#[cfg(test)]
mod tests {
    use async_openai::error::OpenAIError;
    use rstest::rstest;
    use tokio::sync::mpsc;

    use crate::{
        connections::{
            openai::messages::{
                create_assistant_message, create_system_message, create_tool_message,
                create_user_message,
            },
            openai::tools::{collect_tool_call_chunks, Tool},
            OpenAIConnection,
        },
        constant::OPENAI_CHAT_MODEL_MINI,
        log_error,
        tracing::setup::setup_tracing,
    };

    #[tokio::test]
    #[rstest]
    #[case("logs".to_owned(), None, None, 1.0)]
    #[case("Get logs".to_owned(), None, None, 1.0)]
    #[case("Get logs!".to_owned(), None, None, 1.0)]
    #[case("Could you retrieve logs?".to_owned(), None, None, 1.0)]
    #[case("Could you retrieve logs for me?".to_owned(), None, None, 1.0)]
    #[case("Could you retrieve logs for the cluster for me?".to_owned(), None, None, 1.0)]
    #[case("Could you investigate the logs from logd in hik8s-system?".to_owned(), Some("logd".to_string()), Some("hik8s-system".to_string()), 1.0)]
    #[case("logd logs in hik8s-system?".to_owned(), Some("logd".to_string()), Some("hik8s-system".to_string()), 1.0)]
    #[case("logd logs in hik8s-system for me".to_owned(), Some("logd".to_string()), Some("hik8s-system".to_string()), 1.0)]
    async fn test_completion_log_retrieval(
        #[case] prompt: String,
        #[case] application: Option<String>,
        #[case] namespace: Option<String>,
        #[case] success_rate: f32,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let num_choices = 20;

        // base request
        let messages = vec![create_system_message(), create_user_message(&prompt)];
        let request =
            openai.chat_complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI, num_choices);
        let response = openai.create_completion(request).await?;

        // tool processing
        let mut success_counter: f32 = 0.0;
        for choice in response.choices {
            let tool_calls = choice.message.tool_calls.unwrap_or(Vec::new());
            assert!(tool_calls.len() <= 1, "Expected no more then on tool call.");
            for tool_call in tool_calls {
                let tool = Tool::try_from(tool_call.function).unwrap();
                assert!(matches!(tool, Tool::LogRetrieval(_)));
                if let Tool::LogRetrieval(args) = &tool {
                    assert_eq!(args.application, application);
                    assert_eq!(args.namespace, namespace);
                    assert!(!args.intention.is_empty());
                    success_counter += 1.0;
                }
            }
        }
        let res = success_counter / num_choices as f32;
        assert!(
            res >= success_rate,
            "Average of successful tool call not met: got {res} expected >= {success_rate} | case: {prompt}",
        );
        // let usage = response.usage.unwrap();
        // tracing::info!("completion tokens: {}", usage.completion_tokens);
        // tracing::info!("prompt tokens: {}", usage.prompt_tokens);
        // tracing::info!("total tokens: {}", usage.total_tokens);
        Ok(())
    }

    #[tokio::test]
    async fn test_stream_completion_tools() -> Result<(), OpenAIError> {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        // base request
        let prompt = "I have a problem with my application called logd in namespace hik8s-stag? Could you investigate the logs and also provide an overview of the cluster?";
        let mut messages = vec![create_system_message(), create_user_message(prompt)];
        let request = openai.chat_complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI, 1);
        let stream = openai.create_completion_stream(request).await?;

        let (_, tool_call_chunks) = openai
            .process_completion_stream(&tx, stream)
            .await
            .map_err(|e| log_error!(e))?;

        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }

        assert!(answer.is_empty());
        assert!(!tool_call_chunks.is_empty());

        // tool processing
        let tool_calls = collect_tool_call_chunks(tool_call_chunks);
        messages.push(create_assistant_message(
            "Assistent requested tool calls. Asses the tool calls and make another tool request to cluster overview",
            Some(tool_calls.clone()),
        ));
        for tool_call in tool_calls {
            let tool = Tool::try_from(tool_call.function).unwrap();
            messages.push(create_tool_message(&tool.test_request(), &tool_call.id));
        }

        // tool request
        let request = openai.chat_complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI, 1);
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let stream = openai.create_completion_stream(request).await?;
        let (_, tool_call_chunks) = openai
            .process_completion_stream(&tx, stream)
            .await
            .map_err(|e| log_error!(e))?;

        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        assert!(!answer.is_empty());
        assert!(tool_call_chunks.is_empty());
        Ok(())
    }
}
