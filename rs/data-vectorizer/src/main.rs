use futures_util::StreamExt;
use shared::{
    connections::fluvio::connect::{FluvioConnection, FluvioConnectionError, TOPIC_NAME_CLASS},
    tracing::setup::setup_tracing,
    types::{classification::class::Class, record::consumer_record::ConsumerRecordError},
};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum DataVectorizationError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    // ConsumerRecordError
    #[error("Failed to parse fluvio consumer record: {0}")]
    ConsumerRecordError(#[from] ConsumerRecordError),
}

#[tokio::main]
async fn main() -> Result<(), DataVectorizationError> {
    setup_tracing();
    let fluvio_connection = FluvioConnection::new(&TOPIC_NAME_CLASS.to_owned()).await?;
    let mut consumer = fluvio_connection.create_consumer(1).await?;
    while let Some(Ok(record)) = consumer.next().await {
        let class: Class = record.try_into()?;
        info!("Received class: {:?}", class);
    }

    Ok(())
}
