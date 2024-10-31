use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk, ChatCompletionTool,
    ChatCompletionToolType, FunctionCall, FunctionObject,
};
use serde_json::json;

use std::fmt;

use crate::connections::prompt_engine::connect::{
    AugmentationRequest, PromptEngineConnection, PromptEngineError,
};

pub enum Tool {
    ClusterOverview,
    LogRetrieval,
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tool_name = match self {
            Tool::ClusterOverview => "cluster-overview",
            Tool::LogRetrieval => "log-retrieval",
        };
        write!(f, "{}", tool_name)
    }
}
impl TryFrom<&String> for Tool {
    type Error = String;

    fn try_from(name: &String) -> Result<Self, Self::Error> {
        match name.as_str() {
            "cluster-overview" => Ok(Tool::ClusterOverview),
            "log-retrieval" => Ok(Tool::LogRetrieval),
            _ => Err(format!("Could not parse tool name: {}", name)),
        }
    }
}
impl Tool {
    pub fn get_function(&self) -> FunctionObject {
        match self {
            Tool::ClusterOverview => FunctionObject {
                name: Tool::ClusterOverview.to_string(),
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
            Tool::LogRetrieval => FunctionObject {
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
                    },
                    "additionalProperties": false,
                    "required": ["application", "namespace"]
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
            Tool::LogRetrieval => {
                let request = AugmentationRequest::new(&user_message, &customer_id);
                prompt_engine.request_augmentation(request).await
            },
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
