use fluvio::dataplane::link::ErrorCode;
use fluvio::dataplane::record::ConsumerRecord;
use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use futures_util::{Stream, StreamExt};
use shared::connections::fluvio::connect::ConnectionError;
use shared::{
    connections::fluvio::connect::{FluvioConnection, DEFAULT_TOPIC},
    tracing::setup::setup_tracing,
};
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum ConsumerError {
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
async fn main() -> Result<(), ConsumerError> {
    setup_tracing();

    let mut consumer = create_consumer().await?;

    while let Some(Ok(record)) = consumer.next().await {
        println!("{}", String::from_utf8_lossy(record.as_ref()));
    }

    Ok(())
}

async fn create_consumer(
) -> Result<impl Stream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin, ConsumerError> {
    let fluvio_connection = FluvioConnection::new().await?;

    let consumer = fluvio_connection
        .fluvio
        .consumer_with_config(
            ConsumerConfigExtBuilder::default()
                .topic(DEFAULT_TOPIC.to_string())
                .offset_consumer("my-consumer".to_string())
                .offset_start(Offset::beginning())
                .offset_strategy(OffsetManagementStrategy::Auto)
                .build()
                .map_err(|e| {
                    error!("Error creating consumer config: {}", e);
                    ConsumerError::ConsumerConfigError(e.to_string())
                })?,
        )
        .await
        .map_err(|e| {
            error!("Error creating consumer: {}", e);
            ConsumerError::ConsumerError(e.to_string())
        })?;

    Ok(consumer)
}
