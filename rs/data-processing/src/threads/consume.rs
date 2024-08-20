use crate::ClassificationTask;
use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use futures_util::StreamExt;
use shared::types::record::log::LogRecordError;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info};

use super::types::communication::{ClassificationResult, ConsumerRecordError};

#[derive(Error, Debug)]
pub enum ConsumerThreadError {
    #[error("Failed to send task to worker: {0}")]
    SendError(#[from] mpsc::error::SendError<ClassificationTask>),
    #[error("Failed to parse log record: {0}")]
    LogRecordError(#[from] LogRecordError),
    #[error("Failed to commit offset for key {key}: {source}. ID: {id}")]
    OffsetCommitError {
        key: String,
        source: ErrorCode,
        id: String,
    },
    #[error("Failed to flush offset for key {key}: {source}. ID: {id}")]
    OffsetFlushError {
        key: String,
        source: ErrorCode,
        id: String,
    },
    #[error("Processing failed for key {key}. ID: {id}")]
    ProcessingFailed { key: String, id: String },
    #[error("Failed to parse fluvio consumer record: {0}")]
    ConsumerRecordError(#[from] ConsumerRecordError),
}

enum OffsetErrorType {
    OffsetCommit,
    OffsetFlush,
}

impl From<(OffsetErrorType, ErrorCode, String, String)> for ConsumerThreadError {
    fn from((error_type, source, key, id): (OffsetErrorType, ErrorCode, String, String)) -> Self {
        match error_type {
            OffsetErrorType::OffsetCommit => {
                ConsumerThreadError::OffsetCommitError { key, source, id }
            }
            OffsetErrorType::OffsetFlush => {
                ConsumerThreadError::OffsetFlushError { key, source, id }
            }
        }
    }
}

pub async fn consume_logs(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin,
    sender: mpsc::Sender<ClassificationTask>,
    mut receiver: mpsc::Receiver<ClassificationResult>,
) -> Result<(), ConsumerThreadError> {
    while let Some(Ok(record)) = consumer.next().await {
        let task = ClassificationTask::try_from(record)?;
        // receiver is process thread found in ../process.rs
        sender.send(task).await?;

        if let Some(classification_result) = receiver.recv().await {
            info!(
                "Successfully processed log with key: {}, id: {}",
                classification_result.key, classification_result.log_id
            );
            if let Err(e) = consumer.offset_commit().map_err(|e| {
                ConsumerThreadError::from((
                    OffsetErrorType::OffsetCommit,
                    e,
                    classification_result.key.clone(),
                    classification_result.log_id.clone(),
                ))
            }) {
                // maybe tolerate this error
                return Err(e);
            }
            if let Err(e) = consumer.offset_flush().await.map_err(|e| {
                ConsumerThreadError::from((
                    OffsetErrorType::OffsetFlush,
                    e,
                    classification_result.key,
                    classification_result.log_id,
                ))
            }) {
                return Err(e);
            }
        }
    }
    Ok(())
}
