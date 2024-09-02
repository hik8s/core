// chat-backend
// pub const OPENAI_CHAT_MODEL: &str = "gpt-3.5-turbo-0125";
pub const CHAT_BACKEND_PORT: &str = "8080";
pub const OPENAI_CHAT_MODEL: &str = "gpt-4-0125-preview";

// prompt-engine
pub const PROMPT_ENGINE_PORT: &str = "8081";
pub const PROMPT_ENGINE_PATH: &str = "prompt";

// qdrant
pub const QDRANT_COLLECTION_LOG: &str = "logs";

// greptime
pub const GREPTIME_DB_NAME: &str = "logs";

// embedding
pub const OPENAI_EMBEDDING_MODEL: &str = "text-embedding-3-large";
pub const OPENAI_EMBEDDING_TOKEN_LIMIT: usize = 100000;
pub const EMBEDDING_SIZE: u64 = 3072;
pub const EMBEDDING_USIZE: usize = EMBEDDING_SIZE as usize;
