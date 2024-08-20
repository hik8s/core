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
        let key = task.key.to_owned();
        let id = task.log_record.record_id.to_owned();
        let classes = state.get_or_create(&key).await?;

        match classifier.classify(&task.log_record, classes) {
            Ok(class_id) => {
                let result = ClassificationResult::new(&task, class_id);
                // if this is not in batches, writing single logs with their class is not efficient
                sender
                    .send(result)
                    .await
                    // maybe tolerate this error
                    .map_err(ProcessThreadError::SendError)?;
            }
            Err(e) => error!("{e}: key: {key}, log id: {id}"),
        }
    }
    Ok(())
}
