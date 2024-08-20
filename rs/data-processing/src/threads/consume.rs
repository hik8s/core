use crate::ClassificationTask;
use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use futures_util::StreamExt;
use shared::connections::fluvio::connect::{commit_and_flush_offsets, OffsetError};
use shared::types::record::log::LogParseError;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info};

use super::types::communication::{ClassificationResult, ConsumerRecordError};

#[derive(Error, Debug)]
pub enum ConsumerThreadError {
    #[error("Failed to send task to worker: {0}")]
    SendError(#[from] mpsc::error::SendError<ClassificationTask>),
    #[error("Failed to parse log record: {0}")]
    LogRecordError(#[from] LogParseError),
    #[error("Fluvio offset error: {0}")]
    OffsetError(#[from] OffsetError),
    #[error("Failed to parse fluvio consumer record: {0}")]
    ConsumerRecordError(#[from] ConsumerRecordError),
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
            commit_and_flush_offsets(
                &mut consumer,
                classification_result.key.clone(),
                classification_result.log_id.clone(),
            )
            .await?;
        }
    }
    Ok(())
}
