use thiserror::Error;

use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};

#[derive(Error, Debug)]
pub enum OffsetError {
    #[error("Failed to commit offset for key {1}: {0}. ID: {2}")]
    Commit(ErrorCode, String, String),
    #[error("Failed to flush offset for key {1}: {0}. ID: {2}")]
    Flush(ErrorCode, String, String),
}

pub async fn commit_and_flush_offsets(
    consumer: &mut (impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin),
    key: String,
    id: String,
) -> Result<(), OffsetError> {
    consumer
        .offset_commit()
        .map_err(|e| OffsetError::Commit(e, key.clone(), id.clone()))?;
    consumer
        .offset_flush()
        .await
        .map_err(|e| OffsetError::Flush(e, key, id))?;
    Ok(())
}
