use crate::ClassificationTask;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use futures_util::{Stream, StreamExt};
use serde_json::from_str;
use shared::types::parsedline::ParsedLine;
use tokio::sync::mpsc;
use tracing::error;

pub async fn consume_logs(
    mut consumer: impl Stream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin,
    sender: mpsc::Sender<ClassificationTask>,
) {
    while let Some(Ok(record)) = consumer.next().await {
        let payload = record.value().to_vec();
        let key = record.key().map(|k| k.to_vec());

        let data_str = String::from_utf8_lossy(&payload);
        let key_str = String::from_utf8_lossy(&key.unwrap_or_default()).to_string();

        match from_str::<ParsedLine>(&data_str) {
            Ok(parsed_line) => {
                let task = ClassificationTask {
                    parsed_line,
                    key: key_str,
                };
                if let Err(e) = sender.send(task).await {
                    error!("Failed to send task to worker: {}", e);
                }
            }
            Err(e) => error!("Failed to deserialize record: {}", e),
        }
    }
}
