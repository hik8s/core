use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use fluvio::RecordKey;
use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use fluvio::{spu::SpuSocketPool, Fluvio, TopicProducer};
use serde_json::to_string;
use std::sync::Arc;
use tracing::info;

use crate::constant::FLUVIO_BATCH_SIZE;
use crate::types::metadata::Metadata;
use crate::types::record::log::LogRecord;

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
            .map_err(FluvioConnectionError::from)?;

        let admin = fluvio.admin().await;
        create_topic(admin, &topic).await?;

        let producer = fluvio
            .topic_producer(topic.name.to_owned())
            .await
            .map_err(FluvioConnectionError::from)?;

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
                    .offset_consumer(self.topic.consumer_id(partition_id))
                    .offset_start(Offset::beginning())
                    .offset_strategy(OffsetManagementStrategy::Manual)
                    .build()
                    .map_err(|e| FluvioConnectionError::ConsumerConfigError(e.to_string()))?,
            )
            .await
            .map_err(|e| FluvioConnectionError::ConsumerError(e.to_string()))?;
        info!(
            "Consumer '{}' created",
            self.topic.consumer_id(partition_id)
        );
        Ok(consumer)
    }

    pub async fn send_batch(
        &self,
        logs: Vec<LogRecord>,
        metadata: &Metadata,
    ) -> Result<(), FluvioConnectionError> {
        let mut batch = Vec::with_capacity(FLUVIO_BATCH_SIZE);

        for log in &logs {
            let serialized_record = to_string(&log).expect("Failed to serialize record");
            batch.push((
                RecordKey::from(metadata.pod_name.to_owned()),
                serialized_record,
            ));

            if batch.len() == FLUVIO_BATCH_SIZE {
                self.producer
                    .send_all(batch.drain(..))
                    .await
                    .map_err(FluvioConnectionError::Anyhow)?;
            }
        }

        // Send any remaining logs in the batch
        if !batch.is_empty() {
            self.producer
                .send_all(batch.drain(..))
                .await
                .map_err(FluvioConnectionError::Anyhow)?;
        }

        // Ensure the producer flushes the messages
        self.producer
            .flush()
            .await
            .map_err(FluvioConnectionError::Anyhow)?;

        Ok(())
    }
}
