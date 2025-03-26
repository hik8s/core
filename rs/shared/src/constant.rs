// chat-backend
pub const CHAT_BACKEND_PORT: &str = "8080";
pub const OPENAI_CHAT_MODEL: &str = "gpt-4o-2024-08-06";
pub const OPENAI_CHAT_MODEL_MINI: &str = "gpt-4o-mini-2024-07-18";
pub const DEFAULT_ITERATION_DEPTH: usize = 10;

// logs
pub const LOG_PREFIX: &str = "logs";
pub const DATA_INTAKE_LIMIT_MEMIBYTES: u64 = 32;

// embedding
pub const OPENAI_EMBEDDING_MODEL: &str = "text-embedding-3-large";
pub const OPENAI_EMBEDDING_TOKEN_LIMIT: usize = 800000;
pub const EMBEDDING_SIZE: u64 = 3072;
pub const EMBEDDING_USIZE: usize = EMBEDDING_SIZE as usize;

// fluvio
pub const FLUVIO_BYTES_SAFTY_MARGIN: usize = 2048;

pub const TOPIC_LOG_NAME: &str = "logs";
pub const TOPIC_LOG_PARTITIONS: u32 = 2;
pub const TOPIC_LOG_REPLICAS: u32 = 1;
pub const TOPIC_LOG_BYTES_PER_RECORD: usize = 32768;

pub const TOPIC_CLASS_NAME: &str = "classes";
pub const TOPIC_CLASS_PARTITIONS: u32 = 1;
pub const TOPIC_CLASS_REPLICAS: u32 = 1;
pub const TOPIC_CLASS_BYTES_PER_RECORD: usize = TOPIC_LOG_BYTES_PER_RECORD * 4;

pub const TOPIC_EVENT_NAME: &str = "event";
pub const TOPIC_EVENT_PARTITIONS: u32 = 1;
pub const TOPIC_EVENT_REPLICAS: u32 = 1;
pub const TOPIC_EVENT_BYTES_PER_RECORD: usize = 131072;

pub const TOPIC_PROCESSED_EVENT_NAME: &str = "processedevent";
pub const TOPIC_PROCESSED_RESOURCE_NAME: &str = "processedresource";
pub const TOPIC_PROCESSED_CUSTOM_RESOURCE_NAME: &str = "processedcustomresource";

pub const TOPIC_RESOURCE_NAME: &str = "resource";
pub const TOPIC_RESOURCE_PARTITIONS: u32 = 1;
pub const TOPIC_RESOURCE_REPLICAS: u32 = 1;
pub const TOPIC_RESOURCE_BYTES_PER_RECORD: usize = 131072;

pub const TOPIC_CUSTOM_RESOURCE_NAME: &str = "customresource";
pub const TOPIC_CUSTOM_RESOURCE_PARTITIONS: u32 = 1;
pub const TOPIC_CUSTOM_RESOURCE_REPLICAS: u32 = 1;
pub const TOPIC_CUSTOM_RESOURCE_BYTES_PER_RECORD: usize = 131072;

// greptime
pub const GREPTIME_TABLE_KEY: &str = "Tables";
pub const DEFAULT_NS: &str = "NON4MESPACE";
pub const DEFAULT_NAME: &str = "NON4ME";
pub const DEFAULT_KIND: &str = "NOK1ND";

// prompting
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are an assistant to a site reliability engineer. \
You can request tools that allow you to answer questions with relevant data.
If the user does not specify a namespace or application, assume that they want to search the entire cluster. \
Do not ask any questions you are instructed to assume to search the entire cluster in this case. \
For creating a deployment, the user must specify the namespace and application.
";

// #[cfg(test)]
pub const CONVERSION_BYTE_TO_MEBIBYTE: usize = 1048576;
