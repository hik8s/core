use thiserror::Error;
use tokio::sync::mpsc;
use tracing::error;

use super::types::communication::{ClassificationResult, ClassificationTask};

use log_classification::classifier::Classifier;
use log_classification::types::state::{ClassifierState, ClassifierStateError};

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
        let classes = state.get_or_create(&task.key).await?;
        classifier.classify(&task.parsed_line, classes).await;
        let success = true;
        let result = ClassificationResult {
            key: task.key,
            success,
            id: task.parsed_line.id,
        };
        sender
            .send(result)
            .await
            // maybe tolerate this error
            .map_err(ProcessThreadError::SendError)?;
    }
    Ok(())
}
