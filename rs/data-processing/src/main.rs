use fluvio::dataplane::link::ErrorCode;
use fluvio::dataplane::record::ConsumerRecord;
use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use futures_util::{Stream, StreamExt};
use shared::{
    connections::fluvio::connect::{FluvioConnection, DEFAULT_TOPIC},
    tracing::setup::setup_tracing,
};
use tracing::error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_tracing();

    let mut consumer = create_consumer().await?;

    while let Some(Ok(record)) = consumer.next().await {
        println!("{}", String::from_utf8_lossy(record.as_ref()));
    }

    Ok(())
}

async fn create_consumer(
) -> Result<impl Stream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin, Box<dyn std::error::Error>>
{
    let fluvio = FluvioConnection::new().await.unwrap();

    let consumer = fluvio
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
                    e
                })
                .unwrap(),
        )
        .await
        .map_err(|e| {
            error!("Error creating consumer: {}", e);
            e
        })
        .unwrap();

    Ok(consumer)
}
