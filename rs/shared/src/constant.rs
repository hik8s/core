// chat-backend
pub const CHAT_BACKEND_PORT: &str = "8080";
pub const OPENAI_CHAT_MODEL: &str = "gpt-4o-2024-08-06";
pub const OPENAI_CHAT_MODEL_MINI: &str = "gpt-4o-mini-2024-07-18";

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

// prompting
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are a site reliability engineer in a large company. \
You get a request from a fellow engineer and you need to understand the key information";

// #[cfg(test)]
pub const CONVERSION_BYTE_TO_MEBIBYTE: usize = 1048576;
