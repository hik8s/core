use crate::{log_error, log_warn_continue};
use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use fluvio::TopicProducerConfigBuilder;
use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use fluvio::{spu::SpuSocketPool, Fluvio, TopicProducer};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;
use tokio::time::timeout;

use super::error::FluvioConnectionError;
use super::topic::{create_topic, FluvioTopic, TopicName};
use super::util::get_record_key;

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
            .map_err(|e| FluvioConnectionError::ClientConnection(log_error!(e)))?;

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
    ) -> Result<impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>, FluvioConnectionError>
    {
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
            .map_err(|e| FluvioConnectionError::ConsumerError(log_error!(e)))?;
        Ok(consumer)
    }

    pub async fn next_batch(
        &self,
        consumer: &mut impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>,
        polling_interval: Duration,
    ) -> Result<HashMap<String, Vec<ConsumerRecord>>, FluvioConnectionError> {
        let start_time = Instant::now();
        let mut batch: HashMap<String, Vec<ConsumerRecord>> = HashMap::new();

        while start_time.elapsed() < polling_interval {
            let result = match timeout(polling_interval, consumer.next()).await {
                Ok(Some(Ok(record))) => Ok(record),
                Ok(Some(Err(e))) => Err(e), // error receiving record
                Ok(None) => continue,       // consumer stream ended (does not happen)
                Err(_) => continue,         // no record received within the timeout
            };
            let record = log_warn_continue!(result);
            let customer_id = get_record_key(&record).map_err(|e| log_error!(e))?;

            if let Some(records_by_key) = batch.get_mut(&customer_id) {
                records_by_key.push(record);
            } else {
                batch.insert(customer_id, vec![record]);
            }
        }
        Ok(batch)
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
        .map_err(|e| FluvioConnectionError::TopicProducer(log_error!(e)))?;
    Ok(producer)
}
