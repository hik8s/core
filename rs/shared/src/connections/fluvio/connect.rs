use anyhow::Error as AnyhowError;
use fluvio::dataplane::link::ErrorCode;
use fluvio::dataplane::record::ConsumerRecord;
use fluvio::{
    consumer::{ConsumerConfigExtBuilder, OffsetManagementStrategy},
    Offset,
};
use fluvio::{
    metadata::topic::TopicSpec, spu::SpuSocketPool, Fluvio, FluvioAdmin, FluvioError, RecordKey,
    TopicProducer,
};
use futures_util::Stream;
use rocket::{request::FromRequest, State};
use std::sync::Arc;
use thiserror::Error;
use tracing::error;

pub const DEFAULT_TOPIC: &str = "logs";
const DEFAULT_PARTITIONS: u32 = 2;

#[derive(Clone)]
pub struct FluvioConnection {
    pub fluvio: Arc<Fluvio>,
    pub admin: Arc<FluvioAdmin>,
    pub producer: Arc<TopicProducer<SpuSocketPool>>,
}

#[derive(Error, Debug)]
pub enum ConnectionError {
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

impl FluvioConnection {
    pub async fn new() -> Result<Self, ConnectionError> {
        let fluvio = Fluvio::connect().await.map_err(ConnectionError::from)?;
        let producer = fluvio
            .topic_producer(DEFAULT_TOPIC.to_string())
            .await
            .map_err(ConnectionError::from)?;
        let admin = fluvio.admin().await;

        let fluvio = Arc::new(fluvio);
        let admin = Arc::new(admin);
        let producer = Arc::new(producer);

        let connection = FluvioConnection {
            fluvio,
            producer,
            admin,
        };

        connection
            .create_topic(DEFAULT_TOPIC, DEFAULT_PARTITIONS)
            .await?;

        Ok(connection)
    }

    pub async fn create_topic(
        &self,
        topic_name: &str,
        partitions: u32,
    ) -> Result<(), ConnectionError> {
        // Check if the topic already exists
        let topics = self.admin.list::<TopicSpec, String>(vec![]).await?;
        if topics.iter().any(|topic| topic.name == topic_name) {
            return Ok(());
        }

        // Create the topic if it does not exist
        let topic_spec = TopicSpec::new_computed(partitions, 1, None);
        self.admin
            .create(topic_name.to_owned(), false, topic_spec)
            .await
            .map_err(ConnectionError::from)
    }

    pub async fn create_consumer(
        &self,
    ) -> Result<impl Stream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin, ConnectionError>
    {
        let consumer = self
            .fluvio
            .consumer_with_config(
                ConsumerConfigExtBuilder::default()
                    .topic(DEFAULT_TOPIC.to_string())
                    // .offset_consumer("my-consumer".to_string())
                    .offset_start(Offset::beginning())
                    .offset_strategy(OffsetManagementStrategy::Auto)
                    .build()
                    .map_err(|e| ConnectionError::ConsumerConfigError(e.to_string()))?,
            )
            .await
            .map_err(|e| ConnectionError::ConsumerError(e.to_string()))?;
        Ok(consumer)
    }
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
