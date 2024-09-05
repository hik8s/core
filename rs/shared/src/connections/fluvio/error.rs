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
}

#[derive(Error, Debug)]
pub enum OffsetError {
    #[error("Failed to commit offset for key {1}: {0}. ID: {2}")]
    Commit(ErrorCode, String, String),
    #[error("Failed to flush offset for key {1}: {0}. ID: {2}")]
    Flush(ErrorCode, String, String),
}
