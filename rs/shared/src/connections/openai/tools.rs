use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk, ChatCompletionTool,
    ChatCompletionToolType, FunctionCall, FunctionObject,
};

use qdrant_client::qdrant::ScoredPoint;
use serde::{Deserialize, Serialize};
use serde_json::json;

use std::fmt;

use crate::{
    connections::{
        dbname::DbName,
        qdrant::{
            connect::{create_filter, create_filter_with_data_type, QdrantConnection},
            error::QdrantConnectionError, EventQdrantMetadata, ResourceQdrantMetadata,
        },
    }, log_error, testdata::UserTestData, types::class::vectorized::VectorizedClass
};

use super::embeddings::request_embedding;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LogRetrievalArgs {
    pub namespace: Option<String>,
    pub application: Option<String>,
    pub intention: String,
}
impl LogRetrievalArgs {
    pub fn new(testdata: &UserTestData) -> Self {
        LogRetrievalArgs {
            namespace: testdata.namespace.to_owned(),
            application: testdata.application.to_owned(),
            intention: "".to_owned(),
        }
    }
}
impl TryFrom<String> for LogRetrievalArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EventRetrievalArgs {
    pub namespace: Option<String>,
    pub application: Option<String>,
    pub kind: Option<String>,
    pub intention: String,
}

impl TryFrom<String> for EventRetrievalArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}

impl EventRetrievalArgs {
    pub fn search_prompt(&self, user_message: &str) -> String {
        let mut prompt: String = user_message.to_string();
        prompt.push_str(format!("\nUser intention: {}", self.intention).as_str());
    
        if self.application.is_some() {
            prompt.push_str(&format!("\nApplication: {}", self.application.as_ref().unwrap()));
        }
        if self.namespace.is_some() {
            prompt.push_str(&format!("\nNamespace: {}", self.namespace.as_ref().unwrap()));
        }
        if self.kind.is_some() {
            prompt.push_str(&format!("\nKind: {}", self.kind.as_ref().unwrap()));
        }
        prompt
    }
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ResourceStatusRetrievalArgs {
    pub namespace: Option<String>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub intention: String,
}

impl TryFrom<String> for ResourceStatusRetrievalArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}

impl ResourceStatusRetrievalArgs {
    pub fn search_prompt(&self, user_message: &str) -> String {
        let mut prompt: String = user_message.to_string();
        prompt.push_str(format!("\nUser intention: {}", self.intention).as_str());
        
        if let Some(kind) = &self.kind {
            prompt.push_str(&format!("\nKind: {}", kind));
        }
        if let Some(name) = &self.name {
            prompt.push_str(&format!("\nName: {}", name));
        }
        if let Some(namespace) = &self.namespace {
            prompt.push_str(&format!("\nNamespace: {}", namespace));
        }
        prompt
    }
}

#[derive(Debug)]
pub enum Tool {
    ClusterOverview,
    LogRetrieval(LogRetrievalArgs),
    EventRetrieval(EventRetrievalArgs),
    ResourceStatusRetrieval(ResourceStatusRetrievalArgs),
    CustomResourceStatusRetrieval(ResourceStatusRetrievalArgs),
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tool_name = match self {
            Tool::ClusterOverview => "cluster-overview",
            Tool::LogRetrieval(_) => "log-retrieval",
            Tool::EventRetrieval(_) => "event-retrieval",
            Tool::ResourceStatusRetrieval(_) => "resource-status-retrieval",
            Tool::CustomResourceStatusRetrieval(_) => "customresource-status-retrieval",
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
            "event-retrieval" => Ok(Tool::EventRetrieval(arguments.try_into().unwrap())),
            "resource-status-retrieval" => Ok(Tool::ResourceStatusRetrieval(arguments.try_into().unwrap())),
            "customresource-status-retrieval" => Ok(Tool::CustomResourceStatusRetrieval(arguments.try_into().unwrap())),
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
                // No args are used, because this is used to instruct the model on how to call the tool.
                // Args are then filled by the model
                // TODO: derive the parameters from the args, e.g. special serialization impl
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
            Tool::EventRetrieval(_) => FunctionObject {
                name: self.to_string(),
                description: Some("Retrieve events from the kubernetes cluster".to_string()),
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
                        "kind": {
                            "type": ["string", "null"],
                            "description": "Name of the resource kind"
                        },
                        "intention": {
                            "type": "string",
                            "description": "The users intention. What does the user want to achieve and what information should the events contain?"
                        }
                    },
                    "additionalProperties": false,
                    "required": ["application", "namespace", "kind", "intention"]
                })),
                strict: Some(true),
            },
            Tool::ResourceStatusRetrieval(_) => FunctionObject {
                name: self.to_string(),
                description: Some("Retrieve the status key of resources from the kubernetes cluster".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": ["string", "null"],
                            "description": "Name of the resource or associated application, service, or container",
                        },
                        "namespace": {
                            "type": ["string", "null"],
                            "description": "Name of the namespace"
                        },
                        "kind": {
                            "type": ["string", "null"],
                            "description": "Resource kind"
                        },
                        "intention": {
                            "type": "string",
                            "description": "The users intention. What does the user want to achieve and what information should the resource status contain?"
                        }
                    },
                    "additionalProperties": false,
                    "required": ["name", "namespace", "kind", "intention"]
                })),
                strict: Some(true),
            },
            Tool::CustomResourceStatusRetrieval(_) => FunctionObject {
                name: self.to_string(),
                description: Some("Retrieve the status key of custom resources from the kubernetes cluster".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": ["string", "null"],
                            "description": "Name of the resource or associated application, service, or container",
                        },
                        "namespace": {
                            "type": ["string", "null"],
                            "description": "Name of the namespace"
                        },
                        "kind": {
                            "type": ["string", "null"],
                            "description": "Custom resource kind"
                        },
                        "intention": {
                            "type": "string",
                            "description": "The users intention. What does the user want to achieve and what information should the resource status contain?"
                        }
                    },
                    "additionalProperties": false,
                    "required": ["name", "namespace", "kind", "intention"]
                })),
                strict: Some(true),
            },
        }
    }
    pub async fn request(
        self,
        qdrant: &QdrantConnection,
        user_message: &str,
        customer_id: &str,
    ) -> Result<String, QdrantConnectionError> {
        match self {
            Tool::ClusterOverview => Ok("We got a bunch of pods here and then theres this huge pile of containers in this corner of the cluster".to_string()),
            Tool::LogRetrieval(args) => {
                let search_prompt = create_search_prompt(user_message, &args);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter(args.namespace.as_ref(), args.application.as_ref());
                let classes = qdrant.search_classes(&DbName::Log, customer_id, array, filter, 30).await?;
                let result = classes
                    .into_iter()
                    .map(|vc| format_log_entry(&vc))
                    .collect::<String>();
                Ok(result)
            }, 
            Tool::EventRetrieval(args) =>   { 
                let search_prompt = args.search_prompt(user_message);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter(None, None);
                let events = qdrant.search_points(&DbName::Event, customer_id, array, filter, 10).await?;
                
                let header = "These are events in the format. Namespace: Object: kind/name, Type: ..., Reason: ..., Message: ..., Score: ...".to_string();
                let result = events
                    .into_iter()
                    .map(|vc| format_event(vc).unwrap_or_default())
                    .collect::<String>();
                Ok(format!("{header}\n{result}"))
            }
            Tool::ResourceStatusRetrieval(args) => {
                let search_prompt = args.search_prompt(user_message);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter_with_data_type(None, None, "status");
                let resource_status = qdrant.search_points(&DbName::Resource, customer_id, array, filter, 10).await?;
                let result = resource_status
                    .into_iter()
                    .map(|sp| format_resource_status(sp).map_err(|e| log_error!(e)).unwrap_or_default())
                    .collect::<String>();
                Ok(result)
            }
            Tool::CustomResourceStatusRetrieval(args) => {
                let search_prompt = args.search_prompt(user_message);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter_with_data_type(None, None, "status");
                let resource_status = qdrant.search_points(&DbName::CustomResource, customer_id, array, filter, 10).await?;
                let result = resource_status
                    .into_iter()
                    .map(|sp| format_resource_status(sp).map_err(|e| log_error!(e)).unwrap_or_default())
                    .collect::<String>();
                Ok(result)
            }
        }
    }
    pub fn test_request(&self) -> String {
        match self {
            Tool::ClusterOverview => "We got a bunch of pods here and then theres this huge pile of containers in this corner of the cluster".to_string(),
            Tool::LogRetrieval(_) => "OOMKilled exit code 137".to_owned(),
            Tool::EventRetrieval(_) => "message: Stopping container hello-server\nreason: Killing".to_owned(),
            Tool::ResourceStatusRetrieval(_) => "Resource status: OOMKilled exit code 137".to_owned(),
            Tool::CustomResourceStatusRetrieval(_) => "Custom resource status: certificate not ready".to_owned(),
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

pub fn create_search_prompt(user_message: &str, args: &LogRetrievalArgs) -> String {
    let mut prompt = user_message.to_string();
    prompt.push_str(format!("\nUser intention: {}", args.intention).as_str());

    prompt
}

pub fn format_log_entry(vc: &VectorizedClass) -> String {
    format!(
        "\n{}/{}, Score {}: {}",
        vc.namespace, vc.key, vc.score, vc.representation
    )
}

pub fn format_event(sp: ScoredPoint) -> Result<String, serde_json::Error> {
    let score = sp.score;
    let event = EventQdrantMetadata::try_from(sp)?;
    Ok(format!(
        "{}: Object: {}/{}, Type: {}, Reason: {}, Message: {}, Score: {}\n",
        event.namespace, event.kind, event.name, event.event_type, event.reason, event.message, score
    ))
}

pub fn format_resource_status(sp: ScoredPoint) -> Result<String, serde_json::Error> {
    let score = sp.score;
    let resource = ResourceQdrantMetadata::try_from(sp)?;

    Ok(format!(
        "{}: Object: {}/{}, Status: {}, Score: {}\n",
        resource.namespace, resource.kind, resource.name, resource.data, score
    ))
}

#[cfg(test)]
mod tests {
    use async_openai::error::OpenAIError;
    use rstest::rstest;
    use tokio::sync::mpsc;

    use crate::{
        connections::{
            openai::{
                messages::{
                    create_assistant_message, create_system_message, create_tool_message,
                    create_user_message,
                },
                tools::{collect_tool_call_chunks, Tool},
            },
            OpenAIConnection,
        },
        constant::OPENAI_CHAT_MODEL_MINI,
        log_error,
        testdata::{UserTest, UserTestData},
        tracing::setup::setup_tracing,
    };

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::Logs), 0.9)]
    #[case(UserTestData::new(UserTest::RetrieveLogs), 0.9)]
    #[case(UserTestData::new(UserTest::RetrieveLogsForMe), 0.9)]
    #[case(UserTestData::new(UserTest::RetrieveLogsForClusterForMe), 0.9)]
    #[case(UserTestData::new(UserTest::RetrieveLogsAppNamespace), 0.9)]
    #[case(UserTestData::new(UserTest::LogsAppNamespace), 0.9)]
    #[case(UserTestData::new(UserTest::LogsAppNamespaceForMe), 0.9)]
    async fn test_completion_log_retrieval(
        #[case] testdata: UserTestData,
        #[case] success_rate: f32,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let num_choices = 20;

        // base request
        let messages = vec![
            create_system_message(),
            create_user_message(&testdata.prompt),
        ];
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
                    assert_eq!(args.application, testdata.application);
                    assert_eq!(args.namespace, testdata.namespace);
                    assert!(!args.intention.is_empty());
                    success_counter += 1.0;
                }
            }
        }
        let res = success_counter / num_choices as f32;
        assert!(
            res >= success_rate,
            "Average of successful tool call not met: got {res} expected >= {success_rate} | prompt: {}",
            testdata.prompt
        );
        // let usage = response.usage.unwrap();
        // tracing::info!("completion tokens: {}", usage.completion_tokens);
        // tracing::info!("prompt tokens: {}", usage.prompt_tokens);
        // tracing::info!("total tokens: {}", usage.total_tokens);
        Ok(())
    }

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::PodKillOutOffMemory))]
    async fn test_stream_completion_tools(
        #[case] testdata: UserTestData,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        // base request
        let mut messages = vec![
            create_system_message(),
            create_user_message(&testdata.prompt),
        ];
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
        tracing::debug!("Messages: {:#?}", messages);
        tracing::debug!("Answer: {}", answer);
        Ok(())
    }
}
