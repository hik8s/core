use shared::{
    connections::{
        error::ConfigError,
        greptime::{
            connect::{GreptimeConnection, GreptimeConnectionError},
            middleware::insert::classified_logs_to_insert_request,
        },
        redis::connect::{RedisConnection, RedisConnectionError},
    },
    preprocessing::log::preprocess_log,
    types::record::classified::ClassifiedLogRecord,
};
use thiserror::Error;
use tokio::sync::mpsc::{self, error::SendError};
use tracing::error;

use super::types::communication::{ClassificationResult, ClassificationTask};

use algorithm::classification::deterministic::classifier::{ClassificationError, Classifier};

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
    #[error("Stream inserter error: {0}")]
    StreamInserterError(#[from] greptimedb_ingester::Error),
    #[error("Failed to initialize Classifier: {0}")]
    ConfigError(#[from] ConfigError),
}

pub async fn process_logs(
    mut receiver: mpsc::Receiver<ClassificationTask>,
    sender: mpsc::Sender<ClassificationResult>,
) -> Result<(), ProcessThreadError> {
    let redis = RedisConnection::new()?;
    let mut classifier = Classifier::new(None, redis)?;
    let greptime = GreptimeConnection::new().await?;
    let stream_inserter = greptime.streaming_inserter()?;
    while let Some(task) = receiver.recv().await {
        let ClassificationTask { key, log } = task;

        // preprocess
        let preprocessed_log = preprocess_log(log);

        // classify
        let class = classifier.classify(&preprocessed_log, &key)?;

        let classified_log = ClassifiedLogRecord::from((preprocessed_log, class));
        let classification_result = ClassificationResult::new(&key, &classified_log);
        let insert_request = classified_logs_to_insert_request(&vec![classified_log], &key);
        stream_inserter.insert(vec![insert_request]).await?;

        // if this is not in batches, writing single logs with their class is not efficient
        sender
            .send(classification_result)
            .await
            // maybe tolerate this error
            .map_err(ProcessThreadError::SendError)?;
    }
    Ok(())
}
