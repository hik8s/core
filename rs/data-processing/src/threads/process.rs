use tokio::sync::mpsc;
use tracing::error;

use super::types::communication::{ClassificationResult, ClassificationTask};

use log_classification::classification::Classifier;

pub async fn process_logs(
    mut receiver: mpsc::Receiver<ClassificationTask>,
    sender: mpsc::Sender<ClassificationResult>,
) {
    let classifier = Classifier::new(None);
    while let Some(task) = receiver.recv().await {
        classifier.classify(&task.parsed_line, task.key);
        let success = true;
        let result = ClassificationResult {
            key: task.key,
            success,
            id: task.parsed_line.id,
        };
        if let Err(e) = sender.send(result).await {
            error!("Failed to send result to main thread: {}", e);
        }
    }
}
