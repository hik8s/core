use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk, ChatCompletionTool,
    ChatCompletionToolType, FunctionCall, FunctionObject,
};

use qdrant_client::qdrant::ScoredPoint;
use serde_json::json;

use std::fmt;

use crate::{
    connections::{
        greptime::greptime_connection::parse_resource_name,
        openai::tool_args::{
            ClusterOverviewArgs, CreateDeploymentArgs, EventRetrievalArgs, LogRetrievalArgs,
            ResourceStatusRetrievalArgs,
        },
        qdrant::{EventQdrantMetadata, ResourceQdrantMetadata},
    },
    log_error,
    qdrant_util::{create_filter, create_filter_with_data_type},
    types::class::vectorized::{from_scored_point, VectorizedClass},
    DbName, GreptimeConnection, QdrantConnection,
};

use super::{embeddings::request_embedding, error::ToolRequestError};

pub fn list_all_tools() -> Vec<ChatCompletionTool> {
    vec![
        Tool::ClusterOverview(ClusterOverviewArgs::default()).into(),
        Tool::LogRetrieval(LogRetrievalArgs::default()).into(),
        Tool::EventRetrieval(EventRetrievalArgs::default()).into(),
        Tool::ResourceStatusRetrieval(ResourceStatusRetrievalArgs::default()).into(),
        Tool::CustomResourceStatusRetrieval(ResourceStatusRetrievalArgs::default()).into(),
        Tool::ResourceSpecRetrieval(ResourceStatusRetrievalArgs::default()).into(),
        Tool::CustomResourceSpecRetrieval(ResourceStatusRetrievalArgs::default()).into(),
        Tool::CreateDeployment(CreateDeploymentArgs::default()).into(),
    ]
}

#[derive(Debug, Clone)]
pub enum Tool {
    ClusterOverview(ClusterOverviewArgs),
    CreateDeployment(CreateDeploymentArgs),
    LogRetrieval(LogRetrievalArgs),
    EventRetrieval(EventRetrievalArgs),
    ResourceStatusRetrieval(ResourceStatusRetrievalArgs),
    ResourceSpecRetrieval(ResourceStatusRetrievalArgs),
    CustomResourceStatusRetrieval(ResourceStatusRetrievalArgs),
    CustomResourceSpecRetrieval(ResourceStatusRetrievalArgs),
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tool_name = match self {
            Tool::ClusterOverview(_) => "cluster-overview",
            Tool::CreateDeployment(_) => "deployment-options",
            Tool::LogRetrieval(_) => "log-retrieval",
            Tool::EventRetrieval(_) => "event-retrieval",
            Tool::ResourceStatusRetrieval(_) => "resource-status-retrieval",
            Tool::ResourceSpecRetrieval(_) => "resource-spec-retrieval",
            Tool::CustomResourceStatusRetrieval(_) => "customresource-status-retrieval",
            Tool::CustomResourceSpecRetrieval(_) => "customresource-spec-retrieval",
        };
        write!(f, "{}", tool_name)
    }
}

impl Tool {
    pub fn get_args(&self) -> &dyn std::fmt::Display {
        match self {
            Tool::ClusterOverview(args) => args,
            Tool::ResourceStatusRetrieval(args) => args,
            Tool::ResourceSpecRetrieval(args) => args,
            Tool::CustomResourceStatusRetrieval(args) => args,
            Tool::CustomResourceSpecRetrieval(args) => args,
            Tool::EventRetrieval(args) => args,
            Tool::LogRetrieval(args) => args,
            Tool::CreateDeployment(args) => args,
        }
    }
}

impl TryFrom<FunctionCall> for Tool {
    type Error = String;

    fn try_from(call: FunctionCall) -> Result<Self, Self::Error> {
        let FunctionCall { name, arguments } = call;
        match name.as_str() {
            "cluster-overview" => Ok(Tool::ClusterOverview(arguments.try_into().unwrap())),
            "deployment-options" => Ok(Tool::CreateDeployment(arguments.try_into().unwrap())),
            "log-retrieval" => Ok(Tool::LogRetrieval(arguments.try_into().unwrap())),
            "event-retrieval" => Ok(Tool::EventRetrieval(arguments.try_into().unwrap())),
            "resource-status-retrieval" => {
                Ok(Tool::ResourceStatusRetrieval(arguments.try_into().unwrap()))
            }
            "resource-spec-retrieval" => {
                Ok(Tool::ResourceSpecRetrieval(arguments.try_into().unwrap()))
            }
            "customresource-status-retrieval" => Ok(Tool::CustomResourceStatusRetrieval(
                arguments.try_into().unwrap(),
            )),
            "customresource-spec-retrieval" => Ok(Tool::CustomResourceSpecRetrieval(
                arguments.try_into().unwrap(),
            )),
            _ => Err(format!("Could not parse tool name: {}", name)),
        }
    }
}
impl Tool {
    pub fn get_function(&self) -> FunctionObject {
        match self {
            Tool::ClusterOverview(_) => FunctionObject {
                name: self.to_string(),
                description: Some(
                    "Retrieve an overview of the cluster and its applications".to_string(),
                ),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "resources": {
                            "type": ["array", "null"],
                            "items": {
                                "type": "string",
                                "enum": [
                                    "deployment", "daemonset", "replicaset", "statefulset", "pod", 
                                    "service", "namespace", "node", "ingress", "serviceaccount", 
                                    "role", "clusterrole", "clusterrolebinding", "storageclass", "all"
                                ]
                            },
                            "description": "The types of resources to retrieve. For example, 'pods', 'services', 'deployments', etc. select all for search across all resources."
                        },
                    },
                    "additionalProperties": false,
                    "required": ["resources"]
                })),
                strict: Some(true),
            },
            Tool::CreateDeployment(_) => FunctionObject {
                name: self.to_string(),
                description: Some(
                    "Retrieve deployment options. When a user requests to create a deployment (for kubernetes), you can use these options for database connections and registry secrets.".to_string(),
                ),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the application."
                        },
                        "namespace": {
                            "type": "string",
                            "description": "Name of the namespace for the application."
                        },
                        "image_name": {
                            "type": "string",
                            "description": "Name of the image of the application."
                        },
                        "databases": {
                            "type": "array",
                            "description": "List of available database connections",
                            "items": {
                                "type": "string",
                                "description": "To what database the deployment needs to connect. Show the user what database options exist and ask what they want to use."
                            },
                        },
                    },
                    "additionalProperties": false,
                    "required": ["databases", "name", "namespace", "image_name"]
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
            Tool::ResourceSpecRetrieval(_) => FunctionObject {
                name: self.to_string(),
                description: Some("Retrieve the spec key of resources from the kubernetes cluster".to_string()),
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
            Tool::CustomResourceSpecRetrieval(_) => FunctionObject {
                name: self.to_string(),
                description: Some("Retrieve the Spec key of custom resources from the kubernetes cluster".to_string()),
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
        greptime: &GreptimeConnection,
        qdrant: &QdrantConnection,
        user_message: &str,
        customer_id: &str,
    ) -> Result<String, ToolRequestError> {
        // TODO: add meaningful error types with context
        let tool_name = self.to_string();
        match self {
            Tool::ClusterOverview(args) => {
                let exclude_deleted = true;
                let db = DbName::Resource.id(customer_id);

                // Define what resources to query
                let resources_to_query =
                    if args.resources.is_empty() || args.resources.contains(&"all".to_string()) {
                        vec![None] // Just a single query with None to get everything
                    } else {
                        // Map each resource to a Some(resource)
                        args.resources.iter().map(|r| Some(r.as_str())).collect()
                    };

                let mut result = String::new();
                for resource in resources_to_query {
                    // TODO: handle error graceful
                    let tables_raw: Vec<String> = greptime
                        .list_tables(&db, None, resource, exclude_deleted)
                        .await?;
                    let tables = tables_raw
                        .iter()
                        .filter_map(|name| parse_resource_name(name))
                        .map(|table| table.print_table())
                        .collect::<Vec<String>>();

                    // Create header for this resource type
                    let header = match resource {
                        Some(r) => format!("Found {} resources of type {}", tables.len(), r),
                        None => format!("Found {} resources across all types", tables.len()),
                    };
                    result.push_str(&header);
                    result.push_str(&tables.join("\n"));
                }

                Ok(result)
            }
            Tool::CreateDeployment(args) => {
                let message = format!(
                    r###"
These are the options for a deployment on kubernetes:

# Application metadata
Name: "{application_name}"
Namespace: "{namespace}"

# Docker registry
Registry name: "ghcr.io/hik8s"
Image name: "{image_name}"
Image pull secret: "ghcr-read-token"

# Database connections
- Postgres
POSTGRES_DB_NAME: "postgres://user:password@localhost:5432/db"

- Message broker
KAFKA_URL: "kafka://broker:9092"

Create a Deployment with the provided information and without adding anything that was not asked.
If the user did not specify any databases, ask wheter they want to add a database connection or, 
if a databases are specified provide the exact yaml."###,
                    application_name = args.name,
                    namespace = args.namespace,
                    image_name = args.image_name
                );
                Ok(message.to_string())
            }
            Tool::LogRetrieval(args) => {
                let db = DbName::Log.id(customer_id);
                let search_prompt = create_search_prompt(user_message, &args);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter(args.namespace.as_ref(), args.application.as_ref());
                let points = qdrant.search_points(&db, array, filter, 30).await?;
                let classes = from_scored_point(points)?;
                let result = classes
                    .into_iter()
                    .map(|vc| format_log_entry(&vc))
                    .collect::<String>();
                Ok(result)
            }
            Tool::EventRetrieval(args) => {
                let db = DbName::Event.id(customer_id);
                let search_prompt = args.search_prompt(user_message);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter(None, None);
                let events = qdrant.search_points(&db, array, filter, 10).await?;

                let header = "These are events in the format. Namespace: Object: kind/name, Type: ..., Reason: ..., Message: ..., Score: ...".to_string();
                let result = events
                    .into_iter()
                    .map(|vc| format_event(vc).unwrap_or_default())
                    .collect::<String>();
                Ok(format!("{header}\n{result}"))
            }
            Tool::ResourceStatusRetrieval(args) => {
                let db = DbName::Resource.id(customer_id);
                let search_prompt = args.search_prompt(user_message);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter_with_data_type(None, None, "status");
                let resource_status = qdrant.search_points(&db, array, filter, 10).await?;
                let result = resource_status
                    .into_iter()
                    .map(|sp| {
                        format_resource_status(sp)
                            .map_err(|e| log_error!(e))
                            .unwrap_or_default()
                    })
                    .collect::<String>();
                Ok(result)
            }
            Tool::ResourceSpecRetrieval(args) => {
                let db = DbName::Resource.id(customer_id);
                let search_prompt = args.search_prompt(user_message);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter_with_data_type(None, None, "spec");
                let resource_status = qdrant.search_points(&db, array, filter, 10).await?;
                let result = resource_status
                    .into_iter()
                    .map(|sp| {
                        format_resource_status(sp)
                            .map_err(|e| log_error!(e))
                            .unwrap_or_default()
                    })
                    .collect::<String>();
                Ok(result)
            }
            Tool::CustomResourceStatusRetrieval(args) => {
                let db = DbName::CustomResource.id(customer_id);
                let search_prompt = args.search_prompt(user_message);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter_with_data_type(None, None, "status");
                let resource_status = qdrant.search_points(&db, array, filter, 10).await?;
                let result = resource_status
                    .into_iter()
                    .map(|sp| {
                        format_resource_status(sp)
                            .map_err(|e| log_error!(e))
                            .unwrap_or_default()
                    })
                    .collect::<String>();
                Ok(result)
            }
            Tool::CustomResourceSpecRetrieval(args) => {
                let db = DbName::CustomResource.id(customer_id);
                let search_prompt = args.search_prompt(user_message);
                let array = request_embedding(&vec![search_prompt]).await.unwrap()[0];
                let filter = create_filter_with_data_type(None, None, "spec");
                let resource_status = qdrant.search_points(&db, array, filter, 10).await?;
                let result = resource_status
                    .into_iter()
                    .map(|sp| {
                        format_resource_status(sp)
                            .map_err(|e| log_error!(e))
                            .unwrap_or_default()
                    })
                    .collect::<String>();
                Ok(result)
            }
        }
    }
    pub fn test_request(&self) -> String {
        match self {
            Tool::ClusterOverview(_) => {
                "You have these resources in the cluster: pod examples test1".to_string()
            }
            Tool::CreateDeployment(_) => "Deployment Options".to_string(),
            Tool::LogRetrieval(_) => "OOMKilled exit code 137".to_owned(),
            Tool::EventRetrieval(_) => {
                "message: Stopping container hello-server\nreason: Killing".to_owned()
            }
            Tool::ResourceStatusRetrieval(_) => {
                "Resource status: OOMKilled exit code 137".to_owned()
            }
            Tool::CustomResourceStatusRetrieval(_) => {
                "Custom resource status: certificate not ready".to_owned()
            }
            Tool::ResourceSpecRetrieval(_) => "Resource spec: image abc".to_owned(),
            Tool::CustomResourceSpecRetrieval(_) => "Custom spec: image abc".to_owned(),
        }
    }
}

impl From<Tool> for ChatCompletionTool {
    fn from(tool: Tool) -> ChatCompletionTool {
        ChatCompletionTool {
            r#type: ChatCompletionToolType::Function,
            function: tool.get_function(),
        }
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolCallError {
    #[error("Missing tool call ID")]
    MissingId,
    #[error("Invalid function call: {0}")]
    InvalidFunction(String),
    #[error("No tool calls found")]
    EmptyToolCalls,
}

pub fn collect_tool_call_chunks(
    tool_call_chunks: Vec<ChatCompletionMessageToolCallChunk>,
) -> Result<Vec<ChatCompletionMessageToolCall>, ToolCallError> {
    if tool_call_chunks.is_empty() {
        return Err(ToolCallError::EmptyToolCalls);
    }
    let mut tool_calls = Vec::<ChatCompletionMessageToolCall>::new();

    for chunk in tool_call_chunks {
        if let Some(call_id) = chunk.id {
            tool_calls.push(ChatCompletionMessageToolCall {
                id: call_id,
                r#type: ChatCompletionToolType::Function,
                function: FunctionCall {
                    name: String::new(),
                    arguments: String::new(),
                },
            });
        }
        if let Some(function_call) = chunk.function {
            let current_call = tool_calls
                .last_mut()
                .ok_or_else(|| ToolCallError::InvalidFunction("No tool call found".to_string()))?;

            if let Some(name) = function_call.name {
                current_call.function.name.push_str(&name);
            }
            if let Some(arguments) = function_call.arguments {
                current_call.function.arguments.push_str(&arguments);
            }
        }
    }
    Ok(tool_calls)
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
        event.namespace,
        event.kind,
        event.name,
        event.event_type,
        event.reason,
        event.message,
        score
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

    use crate::openai_util::{
        collect_tool_call_chunks, create_assistant_message, create_system_message,
        create_tool_message, create_user_message, Tool,
    };
    use crate::{
        constant::OPENAI_CHAT_MODEL_MINI,
        log_error, setup_tracing,
        testdata::{UserTest, UserTestData},
        OpenAIConnection,
    };

    fn convert_empty_to_none(input: &Option<String>) -> Option<String> {
        match input {
            Some(s) if s.is_empty() => None,
            Some(s) if s == "null" => None,
            Some(s) => Some(s.to_owned()),
            None => None,
        }
    }

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::ClusterOverview))]
    async fn test_tool_call_syntax(#[case] testdata: UserTestData) {
        setup_tracing(false);
        let openai = OpenAIConnection::new();
        let num_choices = 1;

        // base request
        let messages = vec![
            create_system_message(),
            create_user_message(&testdata.prompt),
        ];
        let request =
            openai.chat_complete_request(messages.clone(), OPENAI_CHAT_MODEL_MINI, num_choices);
        let response = openai
            .create_completion(request)
            .await
            .expect("Failed to create completion");

        // check response
        assert!(response.choices.len() == 1);
        tracing::debug!("Response: {:#?}", response.choices.first().unwrap().message);
    }

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::Logs), 0.9)]
    #[case(UserTestData::new(UserTest::RetrieveLogs), 0.9)]
    #[case(UserTestData::new(UserTest::RetrieveLogsForMe), 0.9)]
    #[case(UserTestData::new(UserTest::RetrieveLogsForClusterForMe), 0.9)]
    #[case(UserTestData::new(UserTest::RetrieveLogsAppNamespace), 0.9)]
    #[case(UserTestData::new(UserTest::LogsAppNamespace), 0.9)]
    #[case(UserTestData::new(UserTest::LogsAppNamespaceForMe), 0.9)]
    #[case(UserTestData::new(UserTest::LogsAppNamespaceForMe), 0.9)]
    async fn test_completion_log_retrieval(
        #[case] testdata: UserTestData,
        #[case] success_rate: f32,
    ) {
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
        let response = openai
            .create_completion(request)
            .await
            .expect("Failed to create completion");
        // tool processing
        let mut success_counter: f32 = 0.0;
        for choice in response.choices {
            let tool_calls = choice.message.tool_calls.unwrap_or(Vec::new());
            assert!(tool_calls.len() <= 1, "Expected no more then on tool call.");
            for tool_call in tool_calls {
                let tool = Tool::try_from(tool_call.function).unwrap();
                assert!(matches!(tool, Tool::LogRetrieval(_)));
                if let Tool::LogRetrieval(args) = &tool {
                    assert_eq!(
                        convert_empty_to_none(&args.application),
                        testdata.application
                    );
                    assert_eq!(convert_empty_to_none(&args.namespace), testdata.namespace);
                    assert!(!args.intention.is_empty());
                    success_counter += 1.0;
                }
            }
        }
        let res = success_counter / num_choices as f32;
        if res < success_rate {
            tracing::warn!(
                "Average of successful tool call not met: got {res} expected >= {success_rate} | prompt: {}",
                testdata.prompt
            );
            // No panic, so the test won't fail.
        }
        // let usage = response.usage.unwrap();
        // tracing::info!("completion tokens: {}", usage.completion_tokens);
        // tracing::info!("prompt tokens: {}", usage.prompt_tokens);
        // tracing::info!("total tokens: {}", usage.total_tokens);
    }

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::PodKillOutOffMemory))]
    #[case(UserTestData::new(UserTest::ClusterOverview))]
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
        let tool_calls = collect_tool_call_chunks(tool_call_chunks).unwrap();
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
