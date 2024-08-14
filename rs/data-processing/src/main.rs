use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use futures_util::StreamExt;
use shared::{
    connections::fluvio::connect::{FluvioConnection, DEFAULT_TOPIC},
    tracing::setup::setup_tracing,
};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_tracing();

    let fluvio = FluvioConnection::new().await.unwrap();

    let mut consumer = fluvio
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

    while let Some(Ok(record)) = consumer.next().await {
        println!("{}", String::from_utf8_lossy(record.as_ref()));
    }

    Ok(())
}
