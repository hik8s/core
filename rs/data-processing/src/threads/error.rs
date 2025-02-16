use std::str::Utf8Error;

use shared::{
    fluvio::OffsetError, types::classifier::error::ClassifierError, GreptimeConnectionError,
    RedisConnectionError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessThreadError {
    #[error("Classifier error: {0}")]
    Classifier(#[from] ClassifierError),
    #[error("Greptime connection error: {0}")]
    GreptimeConnection(#[from] GreptimeConnectionError),
    #[error("Redis get error: {0}")]
    RedisGet(#[source] RedisConnectionError),
    #[error("Redis set error: {0}")]
    RedisSet(#[source] RedisConnectionError),
    #[error("Redis init error: {0}")]
    RedisInit(#[source] RedisConnectionError),
    // #[error("Stream inserter error: {0}")]
    // StreamInserter(#[from] greptimedb_ingester::Error),
    #[error("Fluvio producer error: {0}")]
    FluvioProducer(#[from] anyhow::Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("Fluvio offset error: {0}")]
    OffsetError(#[from] OffsetError),
    #[error("Invalid json: {0}")]
    InvalidJson(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[source] serde_json::Error),
    #[error("Deserialization error: {0}")]
    DeserializationError(#[source] serde_json::Error),
    #[error("Resource misses field: {0}")]
    MissingField(String),
}
