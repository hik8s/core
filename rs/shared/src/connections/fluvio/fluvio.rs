use crate::log_error;
use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use fluvio::TopicProducerConfigBuilder;
use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use fluvio::{spu::SpuSocketPool, Fluvio, TopicProducer};
use std::collections::HashMap;
use std::sync::Arc;
use strum::IntoEnumIterator;

use super::error::FluvioConnectionError;
use super::topic::{create_topic, FluvioTopic, TopicName};

#[derive(Clone)]
pub struct FluvioConnection {
    pub fluvio: Arc<Fluvio>,
    pub producers: HashMap<TopicName, Arc<TopicProducer<SpuSocketPool>>>,
    pub topics: HashMap<TopicName, FluvioTopic>,
}

impl FluvioConnection {
    pub async fn new() -> Result<Self, FluvioConnectionError> {
        let fluvio = Fluvio::connect()
            .await
            .map_err(|e| FluvioConnectionError::ClientConnection(log_error!(e).into()))?;

        let mut topics = HashMap::new();
        let mut producers = HashMap::new();

        for name in TopicName::iter() {
            let topic = FluvioTopic::new(name);
            let producer = setup_producer(&fluvio, &topic).await?;
            topics.insert(name, topic);
            producers.insert(name, Arc::new(producer));
        }

        Ok(Self {
            fluvio: Arc::new(fluvio),
            producers,
            topics,
        })
    }

    pub fn get_topic(&self, name: TopicName) -> &FluvioTopic {
        self.topics.get(&name).unwrap()
    }
    pub fn get_producer(&self, name: TopicName) -> &Arc<TopicProducer<SpuSocketPool>> {
        self.producers.get(&name).unwrap()
    }

    pub async fn create_consumer(
        &self,
        partition_id: u32,
        name: TopicName,
    ) -> Result<
        impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin,
        FluvioConnectionError,
    > {
        let topic = FluvioTopic::new(name);
        let consumer = self
            .fluvio
            .consumer_with_config(
                ConsumerConfigExtBuilder::default()
                    .topic(&topic.name)
                    .partition(partition_id)
                    .offset_consumer(&topic.name)
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

async fn setup_producer(
    fluvio: &Fluvio,
    topic: &FluvioTopic,
) -> Result<TopicProducer<SpuSocketPool>, FluvioConnectionError> {
    let admin = fluvio.admin().await;
    create_topic(admin, topic)
        .await
        .map_err(|e| log_error!(e))?;

    let topic_config = TopicProducerConfigBuilder::default()
        .batch_size(topic.max_bytes)
        .build()
        .expect("Failed to create topic producer config");

    let producer = fluvio
        .topic_producer_with_config(topic.name.to_owned(), topic_config)
        .await
        .map_err(|e| FluvioConnectionError::TopicProducer(log_error!(e).into()))?;
    Ok(producer)
}
