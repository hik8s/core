use crate::LogRecord;
use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use futures_util::StreamExt;
use shared::connections::fluvio::connect::{commit_and_flush_offsets, OffsetError};
use shared::types::record::consumer_record::ConsumerRecordError;
use shared::types::record::log::LogParseError;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum ConsumerThreadError {
    #[error("Failed to send task to worker: {0}")]
    SendError(#[from] mpsc::error::SendError<LogRecord>),
    #[error("Failed to parse log record: {0}")]
    LogRecordError(#[from] LogParseError),
    #[error("Fluvio offset error: {0}")]
    OffsetError(#[from] OffsetError),
    #[error("Failed to parse fluvio consumer record: {0}")]
    ConsumerRecordError(#[from] ConsumerRecordError),
    #[error("Serde Json error: {0}")]
    DeserializationError(#[from] serde_json::Error),
}

pub async fn consume_logs(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin,
    sender: mpsc::Sender<LogRecord>,
    mut receiver: mpsc::Receiver<(String, String)>,
) -> Result<(), ConsumerThreadError> {
    while let Some(Ok(record)) = consumer.next().await {
        let log = LogRecord::try_from(record)?;
        sender.send(log).await?;

        if let Some((key, record_id)) = receiver.recv().await {
            info!("Successfully inserted classified log with key: {key}, id: {record_id}");
            commit_and_flush_offsets(&mut consumer, key, record_id).await?;
        }
    }
    Ok(())
}
