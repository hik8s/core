use greptimedb_ingester::Error as GreptimeIngestError;
use shared::{
    connections::{
        error::ConfigError,
        fluvio::connect::{FluvioConnection, FluvioConnectionError, TOPIC_NAME_CLASS},
        greptime::{
            connect::{GreptimeConnection, GreptimeConnectionError},
            middleware::insert::classified_logs_to_insert_request,
        },
        redis::connect::{RedisConnection, RedisConnectionError},
    },
    preprocessing::log::preprocess_log,
    types::{
        error::classificationerror::ClassificationError, record::classified::ClassifiedLogRecord,
    },
};
use thiserror::Error;
use tokio::sync::mpsc::{self, error::SendError};
use tracing::error;

use super::types::communication::{ClassificationResult, ClassificationTask};

use algorithm::classification::deterministic::classifier::Classifier;

#[derive(Error, Debug)]
pub enum ProcessThreadError {
    #[error("Classification error: {0}")]
    ClassificationError(#[from] ClassificationError),
    #[error("Other process error: {0}")]
    OtherError(String),
    #[error("Failed to send result to main thread: {0}")]
    SendError(#[from] SendError<ClassificationResult>),
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Stream inserter error: {0}")]
    StreamInserterError(#[from] GreptimeIngestError),
    #[error("Failed to initialize Classifier: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Failed to serialize: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

pub async fn process_logs(
    mut receiver: mpsc::Receiver<ClassificationTask>,
    sender: mpsc::Sender<ClassificationResult>,
) -> Result<(), ProcessThreadError> {
    let redis = RedisConnection::new()?;
    let fluvio = FluvioConnection::new(&TOPIC_NAME_CLASS.to_owned()).await?;
    let mut classifier = Classifier::new(None, redis)?;
    let greptime = GreptimeConnection::new().await?;
    let stream_inserter = greptime.streaming_inserter()?;
    while let Some(task) = receiver.recv().await {
        let ClassificationTask { key, log } = task;

        // preprocess
        let preprocessed_log = preprocess_log(log);

        // classify
        let class = classifier.classify(&preprocessed_log, &key)?;
        let classified_log = ClassifiedLogRecord::new(preprocessed_log, class.clone());

        let classification_result = ClassificationResult::new(&key, &classified_log);

        // insert into greptimedb
        let insert_request = classified_logs_to_insert_request(&vec![classified_log], &key);
        stream_inserter.insert(vec![insert_request]).await?;

        // produce to fluvio
        fluvio
            .producer
            .send(key, TryInto::<String>::try_into(class)?)
            .await?;

        // send result for offset commit
        sender
            // if this is not in batches, writing single logs with their class is not efficient
            .send(classification_result)
            .await
            // maybe tolerate this error
            .map_err(ProcessThreadError::SendError)?;
    }
    Ok(())
}
