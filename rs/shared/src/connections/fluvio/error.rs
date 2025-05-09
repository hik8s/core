use std::str::Utf8Error;

use anyhow::Error;
use fluvio::dataplane::link::ErrorCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FluvioConnectionError {
    #[error("Client connection error: {0}")]
    ClientConnection(#[source] Error),
    #[error("Topic producer error: {0}")]
    TopicProducer(#[source] Error),
    #[error("Producer send error: {0}")]
    ProducerSend(#[source] Error),
    #[error("Producer flush error: {0}")]
    ProducerFlush(#[source] Error),
    #[error("Admin list error: {0}")]
    AdminList(#[source] Error),
    #[error("Admin create error: {0}")]
    AdminCreate(#[source] Error),
    #[error("Consumer config error: {0}")]
    ConsumerConfigError(#[source] Error),
    #[error("Consumer error: {0}")]
    ConsumerError(#[source] Error),
    #[error("Utf8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("Offset commit error: {0}")]
    OffsetCommit(#[source] ErrorCode),
    #[error("Offset flush error: {0}")]
    OffsetFlush(#[source] ErrorCode),
}
