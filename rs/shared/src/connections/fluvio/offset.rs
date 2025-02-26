use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};

use crate::FluvioConnectionError;

pub async fn commit_and_flush_offsets(
    consumer: &mut impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>,
) -> Result<(), FluvioConnectionError> {
    consumer
        .offset_commit()
        .map_err(FluvioConnectionError::OffsetCommit)?;
    consumer
        .offset_flush()
        .await
        .map_err(FluvioConnectionError::OffsetFlush)
}
