use shared::{
    connections::greptime::{
        connect::{GreptimeConnection, GreptimeConnectionError},
        middleware::insert::classified_logs_to_insert_request,
    },
    preprocessing::log::preprocess_log,
    types::record::classified::ClassifiedLogRecord,
};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::error;

use super::types::communication::{ClassificationResult, ClassificationTask};

use algorithm::{
    classification::deterministic::classifier::Classifier,
    types::state::{ClassifierState, ClassifierStateError},
};

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
    // TODO: greptimedb-ingester pr to make error publics
    // #[error("Stream inserter error: {0}")]
    // StreamInserterError(#[from] greptimedb_ingester::error::Error),
}

pub async fn process_logs(
    mut receiver: mpsc::Receiver<ClassificationTask>,
    sender: mpsc::Sender<ClassificationResult>,
) -> Result<(), ProcessThreadError> {
    let state = ClassifierState::new();
    let classifier = Classifier::new(None);
    let connection = GreptimeConnection::new().await?;
    let stream_inserter = connection.greptime.streaming_inserter().unwrap();
    while let Some(task) = receiver.recv().await {
        let ClassificationTask { key, log } = task;
        let class_state = state.get_or_create(&key).await?;

        let preprocessed_log = preprocess_log(log);
        let class = classifier.classify(&preprocessed_log, class_state);
        let classified_log = ClassifiedLogRecord::from((preprocessed_log, class));

        let result =
            ClassificationResult::new(&key, &classified_log.record_id, &classified_log.class_id);

        let insert_request = classified_logs_to_insert_request(&vec![classified_log], &key);
        stream_inserter.insert(vec![insert_request]).await.unwrap();
        // if this is not in batches, writing single logs with their class is not efficient
        sender
            .send(result)
            .await
            // maybe tolerate this error
            .map_err(ProcessThreadError::SendError)?;
    }
    Ok(())
}
