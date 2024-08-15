use tokio::sync::mpsc;
use tracing::{error, info};

use super::types::communication::ClassificationResult;

pub async fn commit_results(mut receiver: mpsc::Receiver<ClassificationResult>) {
    while let Some(result) = receiver.recv().await {
        if result.success {
            // Commit the successfully processed task
            info!("Successfully processed task for key: {}", result.key);
        } else {
            error!("Failed to process task for key: {}", result.key);
        }
    }
}
