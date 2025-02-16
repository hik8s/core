pub mod connections;
pub mod constant;
pub mod mock;
pub mod preprocessing;
pub mod router;
pub mod testdata;
pub mod tracing;
pub mod types;
pub mod utils;

// fluvio
pub mod fluvio {
    pub use crate::connections::fluvio::{
        error::OffsetError, offset::commit_and_flush_offsets, topic::TopicName,
    };
}
pub use crate::connections::fluvio::error::FluvioConnectionError;
pub use crate::connections::fluvio::fluvio_connection::FluvioConnection;

// greptime
pub use crate::connections::greptime::greptime_connection::{
    GreptimeConnection, GreptimeConnectionError,
};

// openai
pub use crate::connections::openai::openai_connection::OpenAIConnection;
pub mod openai_util {
    pub use crate::connections::openai::messages::{
        create_assistant_message, create_system_message, create_tool_message, create_user_message,
        extract_last_user_text_message,
    };
    pub use crate::connections::openai::tools::{collect_tool_call_chunks, Tool};
}

// qdrant
pub use crate::connections::qdrant::error::QdrantConnectionError;
pub use crate::connections::qdrant::qdrant_connection::QdrantConnection;
pub mod qdrant_util {
    pub use crate::connections::qdrant::qdrant_connection::{
        create_filter, create_filter_with_data_type, match_any, parse_qdrant_value,
        string_condition, string_filter, update_deleted_resources,
    };
}

// redis
pub use crate::connections::redis::redis_connection::{RedisConnection, RedisConnectionError};

// util
pub use crate::connections::util::{get_env_var, get_env_var_as_vec};
pub use crate::connections::{dbname::DbName, ConfigError};
pub use crate::tracing::setup_tracing::setup_tracing;
pub use crate::utils::ratelimit::RateLimiter;
