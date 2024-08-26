use anyhow::Error as AnyhowError;
use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use fluvio::{
    metadata::topic::TopicSpec, spu::SpuSocketPool, Fluvio, FluvioAdmin, FluvioError, RecordKey,
    TopicProducer,
};
use rocket::{request::FromRequest, State};
use serde_json::to_string;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};

use crate::types::metadata::Metadata;
use crate::types::record::log::LogRecord;

pub const TOPIC_NAME_LOG: &str = "logs";
pub const TOPIC_NAME_CLASS: &str = "classes";

pub const CONSUMER_PARTITIONS_LOG: u32 = 2;
pub const CONSUMER_PARTITIONS_CLASS: u32 = 1;

pub const BATCH_SIZE: usize = 100;

#[derive(Clone)]
pub struct FluvioConnection {
    pub fluvio: Arc<Fluvio>,
    pub producer: Arc<TopicProducer<SpuSocketPool>>,
    pub topic: FluvioTopic,
}

#[derive(Error, Debug)]
pub enum FluvioConnectionError {
    #[error("Fluvio error: {0}")]
    Fluvio(#[from] FluvioError),
    #[error("Rocket error: {0}")]
    Rocket(String),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] AnyhowError),
    #[error("Consumer config error: {0}")]
    ConsumerConfigError(String),
    #[error("Consumer error: {0}")]
    ConsumerError(String),
}

pub enum TopicName {
    Log,
    Class,
}

#[derive(Clone)]
pub struct FluvioTopic {
    name: String,
    pub partitions: u32,
    pub replicas: u32,
}

impl FluvioTopic {
    pub fn new(topic: TopicName) -> Self {
        match topic {
            TopicName::Log => FluvioTopic {
                name: "logs".to_owned(),
                partitions: 2,
                replicas: 1,
            },
            TopicName::Class => FluvioTopic {
                name: "classes".to_owned(),
                partitions: 1,
                replicas: 1,
            },
        }
    }
    pub fn consumer_id(&self, partition_id: u32) -> String {
        format!("consumer_{}_{}", self.name, partition_id)
    }
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
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        for log in &logs {
            let serialized_record = to_string(&log).expect("Failed to serialize record");
            batch.push((
                create_record_key(metadata.pod_name.to_owned()),
                serialized_record,
            ));

            if batch.len() == BATCH_SIZE {
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

pub async fn create_topic(
    admin: FluvioAdmin,
    topic: &FluvioTopic,
) -> Result<(), FluvioConnectionError> {
    // Check if the topic already exists
    let topics = admin.list::<TopicSpec, String>(vec![]).await?;
    if topics.iter().any(|t| t.name == topic.name) {
        info!("Topic '{}' already exists", topic.name);
        return Ok(());
    }

    // Create the topic if it does not exist
    let topic_spec = TopicSpec::new_computed(topic.partitions, topic.replicas, None);
    admin
        .create(topic.name.to_owned(), false, topic_spec)
        .await
        .map_err(FluvioConnectionError::from)?;
    info!("Topic '{}' created", topic.name);
    Ok(())
}

#[derive(Error, Debug)]
pub enum OffsetError {
    #[error("Failed to commit offset for key {1}: {0}. ID: {2}")]
    Commit(ErrorCode, String, String),
    #[error("Failed to flush offset for key {1}: {0}. ID: {2}")]
    Flush(ErrorCode, String, String),
}

pub async fn commit_and_flush_offsets(
    consumer: &mut (impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin),
    key: String,
    id: String,
) -> Result<(), OffsetError> {
    consumer
        .offset_commit()
        .map_err(|e| OffsetError::Commit(e, key.clone(), id.clone()))?;
    consumer
        .offset_flush()
        .await
        .map_err(|e| OffsetError::Flush(e, key, id))?;
    Ok(())
}

pub fn create_record_key(id: String) -> RecordKey {
    RecordKey::from(id)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for FluvioConnection {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let connection = request.guard::<&State<FluvioConnection>>().await.unwrap();
        rocket::request::Outcome::Success(connection.inner().clone())
    }
}

#[rocket::async_trait]
impl rocket::fairing::Fairing for FluvioConnection {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "Fluvio connection",
            kind: rocket::fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
        Ok(rocket.manage(self.clone()))
    }
}
