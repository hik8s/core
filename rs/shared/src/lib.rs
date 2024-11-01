pub mod connections;
pub mod constant;
pub mod mock;
pub mod preprocessing;
pub mod router;
pub mod tracing;
pub mod types;
pub mod utils;

pub mod fluvio {
    // use shared::fluvio::{...} instead of shared::connections::fluvio::{...,{...}}
    pub use crate::connections::fluvio::{
        error::FluvioConnectionError, error::OffsetError, fluvio::FluvioConnection,
        offset::commit_and_flush_offsets, topic::TopicName,
    };
}
pub use crate::connections::get_db_name;
pub use crate::connections::get_env_var;
