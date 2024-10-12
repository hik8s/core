use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use fluvio::{spu::SpuSocketPool, Fluvio, TopicProducer};
use fluvio::{Compression, TopicProducerConfigBuilder};
use std::sync::Arc;

use crate::log_error;

use super::error::FluvioConnectionError;
use super::topic::{create_topic, FluvioTopic, TopicName};

#[derive(Clone)]
pub struct FluvioConnection {
    pub fluvio: Arc<Fluvio>,
    pub producer: Arc<TopicProducer<SpuSocketPool>>,
    pub topic: FluvioTopic,
}

impl FluvioConnection {
    pub async fn new(topic_name: TopicName) -> Result<Self, FluvioConnectionError> {
        let topic = FluvioTopic::new(topic_name);

        let fluvio = Fluvio::connect()
            .await
            .map_err(|e| FluvioConnectionError::ClientConnection(log_error!(e).into()))?;

        let admin = fluvio.admin().await;
        create_topic(admin, &topic).await?;

        let topic_config = TopicProducerConfigBuilder::default()
            .compression(Compression::Zstd)
            .batch_size(topic.max_bytes)
            .build()
            .expect("Failed to create topic producer config");

        let producer = fluvio
            .topic_producer_with_config(topic.name.to_owned(), topic_config)
            .await
            .map_err(|e| FluvioConnectionError::TopicProducer(log_error!(e).into()))?;

        let fluvio = Arc::new(fluvio);
        let producer = Arc::new(producer);

        Ok(Self {
            fluvio,
            producer,
            topic,
        })
    }

    pub async fn create_consumer(
        &self,
        partition_id: u32,
    ) -> Result<
        impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin,
        FluvioConnectionError,
    > {
        let consumer = self
            .fluvio
            .consumer_with_config(
                ConsumerConfigExtBuilder::default()
                    .topic(self.topic.name.to_owned())
                    .partition(partition_id)
                    .offset_consumer(self.topic.name.to_owned())
                    .offset_start(Offset::beginning())
                    .offset_strategy(OffsetManagementStrategy::Manual)
                    .build()
                    .map_err(|e| {
                        FluvioConnectionError::ConsumerConfigError(log_error!(e).into())
                    })?,
            )
            .await
            .map_err(|e| FluvioConnectionError::ConsumerError(log_error!(e).into()))?;
        Ok(consumer)
    }
}
