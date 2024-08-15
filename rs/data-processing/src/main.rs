use futures_util::StreamExt;
use serde_json::from_str;
use shared::connections::fluvio::connect::ConnectionError;
use shared::types::parsedline::ParsedLine;
use shared::{connections::fluvio::connect::FluvioConnection, tracing::setup::setup_tracing};
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] ConnectionError),
    #[error("Consumer config error: {0}")]
    ConsumerConfigError(String),
    #[error("Consumer error: {0}")]
    ConsumerError(String),
    #[error("Other error: {0}")]
    Other(#[from] Box<dyn std::error::Error>),
}
#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing();

    let fluvio_connection = FluvioConnection::new().await?;

    let mut consumer = fluvio_connection.create_consumer().await?;

    while let Some(Ok(record)) = consumer.next().await {
        let payload = record.value();
        let data_str = String::from_utf8_lossy(payload);
        match from_str::<ParsedLine>(&data_str) {
            Ok(parsed_line) => println!("{:?}", parsed_line),
            Err(e) => error!("Failed to deserialize record: {}", e),
        }
    }
    Ok(())
}
