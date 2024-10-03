use std::str::{from_utf8, Utf8Error};

use fluvio::dataplane::record::ConsumerRecord;
use greptimedb_ingester::Error as GreptimeIngestError;
use shared::{
    connections::{
        greptime::{
            connect::{GreptimeConnection, GreptimeConnectionError},
            middleware::insert::classified_log_to_insert_request,
        },
        redis::connect::{RedisConnection, RedisConnectionError},
    },
    fluvio::{FluvioConnection, FluvioConnectionError},
    types::{
        classifier::error::ClassifierError,
        record::{log::LogRecord, preprocessed::PreprocessedLogRecord},
    },
};
use thiserror::Error;
use tokio::sync::mpsc::{self, error::SendError};
use tracing::error;

use algorithm::classification::deterministic::classifier::Classifier;

#[derive(Error, Debug)]
pub enum ProcessThreadError {
    #[error("Classifier error: {0}")]
    ClassifierError(#[from] ClassifierError),
    #[error("Failed to send result to main thread: {0}")]
    SendError(#[from] SendError<(String, String)>),
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Stream inserter error: {0}")]
    StreamInserterError(#[from] GreptimeIngestError),
    #[error("Failed to serialize: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Fluvio producer error: {0}")]
    FluvioProducerError(#[from] anyhow::Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
}

pub async fn process_logs(
    mut receiver: mpsc::Receiver<ConsumerRecord>,
    sender: mpsc::Sender<(String, String)>,
    fluvio: FluvioConnection,
) -> Result<(), ProcessThreadError> {
    let redis = RedisConnection::new()?;
    let mut classifier = Classifier::new(None, redis)?;
    let greptime = GreptimeConnection::new().await?;
    while let Some(record) = receiver.recv().await {
        let customer_id = get_record_key(&record)?;
        let log = LogRecord::try_from(record)?;

        // preprocess
        let record_id = log.record_id.to_owned();
        let preprocessed_log = PreprocessedLogRecord::from(log);

        // classify
        let (updated_class, classified_log) = classifier.classify(&preprocessed_log)?;

        // insert into greptimedb
        let insert_request = classified_log_to_insert_request(classified_log);
        let stream_inserter = greptime.streaming_inserter(&customer_id)?;
        stream_inserter.insert(vec![insert_request]).await?;

        // produce to fluvio
        if updated_class.is_some() {
            let class = updated_class.unwrap();
            fluvio
                .producer
                .send(customer_id.clone(), TryInto::<String>::try_into(class)?)
                .await?;
        }

        // send result for offset commit
        sender
            .send((customer_id, record_id))
            .await
            .map_err(ProcessThreadError::SendError)?;
    }
    Ok(())
}

fn get_record_key(record: &ConsumerRecord) -> Result<String, Utf8Error> {
    let key = record.key().unwrap();
    let key = from_utf8(key).map_err(|e| {
        error!("{e}");
        e
    })?;
    Ok(key.to_owned())
}
