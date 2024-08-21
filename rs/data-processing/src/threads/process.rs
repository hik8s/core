use shared::preprocessing::log::preprocess_log;
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
}

pub async fn process_logs(
    mut receiver: mpsc::Receiver<ClassificationTask>,
    sender: mpsc::Sender<ClassificationResult>,
) -> Result<(), ProcessThreadError> {
    let state = ClassifierState::new();
    let classifier = Classifier::new(None);
    while let Some(task) = receiver.recv().await {
        let ClassificationTask { key, log } = task;
        let record_id = log.record_id.to_owned();
        let class_state = state.get_or_create(&key).await?;

        let preprocessed_log = preprocess_log(log);
        let class = classifier.classify(&preprocessed_log, class_state);
        let result = ClassificationResult::new(&key, &record_id, class.id);
        // if this is not in batches, writing single logs with their class is not efficient
        sender
            .send(result)
            .await
            // maybe tolerate this error
            .map_err(ProcessThreadError::SendError)?;
    }
    Ok(())
}
