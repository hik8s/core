use crate::ClassificationTask;
use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use futures_util::StreamExt;
use shared::types::parsedline::{ParsedLine, ParsedLineError};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info};

use super::types::communication::ClassificationResult;

#[derive(Error, Debug)]
pub enum ConsumerThreadError {
    #[error("Failed to send task to worker: {0}")]
    SendError(#[from] mpsc::error::SendError<ClassificationTask>),
    #[error("Failed to parse line: {0}")]
    ParsedLineError(#[from] ParsedLineError),
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
        let payload = record.value().to_vec();
        let key = record.key().map(|k| k.to_vec());

        let data_str = String::from_utf8_lossy(&payload);
        let key_str = String::from_utf8_lossy(&key.unwrap_or_default()).to_string();

        match ParsedLine::from_str(&data_str) {
            Ok(parsed_line) => {
                let task = ClassificationTask {
                    parsed_line,
                    key: key_str,
                };
                sender.send(task).await?;
            }
            Err(e) => error!("{e}"), // tolerate potential parsing errors
        }

        if let Some(ClassificationResult { key, success, id }) = receiver.recv().await {
            if success {
                info!("Successfully processed log with key: {}, id: {}", key, id);
                let idc = id.clone();
                if let Err(e) = consumer.offset_commit().map_err(|e| {
                    ConsumerThreadError::from((OffsetErrorType::OffsetCommit, e, key.clone(), idc))
                }) {
                    // maybe tolerate this error
                    return Err(e);
                }
                if let Err(e) = consumer.offset_flush().await.map_err(|e| {
                    ConsumerThreadError::from((OffsetErrorType::OffsetFlush, e, key.clone(), id))
                }) {
                    return Err(e);
                }
            } else {
                return Err(ConsumerThreadError::ProcessingFailed { key, id });
            }
        }
    }
    Ok(())
}
