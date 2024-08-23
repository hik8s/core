use shared::{
    connections::{
        greptime::{
            connect::{GreptimeConnection, GreptimeConnectionError},
            middleware::insert::classified_logs_to_insert_request,
        },
        redis::connect::{RedisConnection, RedisConnectionError},
    },
    preprocessing::log::preprocess_log,
    types::{
        classification::state::{ClassifierState, ClassifierStateError},
        record::classified::ClassifiedLogRecord,
    },
};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::error;

use super::types::communication::{ClassificationResult, ClassificationTask};

use algorithm::classification::deterministic::classifier::Classifier;

#[derive(Error, Debug)]
pub enum ProcessThreadError {
    #[error("Classifier state error: {0}")]
    ClassifierStateError(#[from] ClassifierStateError),
    #[error("Other process error: {0}")]
    OtherError(String),
    #[error("Failed to send result to main thread: {0}")]
    SendError(#[from] mpsc::error::SendError<ClassificationResult>),
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    // TODO: greptimedb-ingester pr to make error publics
    // #[error("Stream inserter error: {0}")]
    // StreamInserterError(#[from] greptimedb_ingester::error::Error),
}

pub async fn process_logs(
    mut receiver: mpsc::Receiver<ClassificationTask>,
    sender: mpsc::Sender<ClassificationResult>,
) -> Result<(), ProcessThreadError> {
    let state = ClassifierState::new();
    let redis_connection = RedisConnection::new()?;
    let classifier = Classifier::new(None);
    let connection = GreptimeConnection::new().await?;
    let stream_inserter = connection.greptime.streaming_inserter().unwrap();
    while let Some(task) = receiver.recv().await {
        let ClassificationTask { key, log } = task;

        // preprocess
        let preprocessed_log = preprocess_log(log);

        // classify
        let mut class_state = state.get_or_create(&key).await?;
        let class = classifier.classify(&preprocessed_log, &mut class_state);
        state.insert(&key, class_state).await?;
        let classified_log = ClassifiedLogRecord::from((preprocessed_log, class));

        let classification_result = ClassificationResult::new(&key, &classified_log);

        let insert_request = classified_logs_to_insert_request(&vec![classified_log], &key);
        stream_inserter.insert(vec![insert_request]).await.unwrap();

        // if this is not in batches, writing single logs with their class is not efficient
        sender
            .send(classification_result)
            .await
            // maybe tolerate this error
            .map_err(ProcessThreadError::SendError)?;
    }
    Ok(())
}
