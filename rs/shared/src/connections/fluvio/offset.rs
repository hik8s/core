use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};

use super::error::OffsetError;

pub async fn commit_and_flush_offsets(
    consumer: &mut (impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin),
    key: String,
) -> Result<(), OffsetError> {
    consumer
        .offset_commit()
        .map_err(|e| OffsetError::Commit(e, key.clone()))?;
    consumer
        .offset_flush()
        .await
        .map_err(|e| OffsetError::Flush(e, key))?;
    Ok(())
}
