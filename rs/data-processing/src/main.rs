use futures_util::StreamExt;
use logs::classification::Classifier;
use logs::types::appstate::AppStateError;
use serde_json::from_str;
use shared::connections::fluvio::connect::ConnectionError;
use shared::types::parsedline::ParsedLine;
use shared::{connections::fluvio::connect::FluvioConnection, tracing::setup::setup_tracing};
use thiserror::Error;
use tracing::{error, info};

pub mod logs;

use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] ConnectionError),
    #[error("Other error: {0}")]
    Other(#[from] Box<dyn std::error::Error>),
    #[error("App state error: {0}")]
    AppStateError(#[from] AppStateError),
}

struct ClassificationTask {
    parsed_line: ParsedLine,
    key: String,
}

struct ClassificationResult {
    key: String,
    success: bool,
}

#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing();

    let fluvio_connection = FluvioConnection::new().await?;
    let mut consumer = fluvio_connection.create_consumer().await?;

    // Create channels for task submission and result collection
    let (task_tx, mut task_rx) = mpsc::channel::<ClassificationTask>(100);
    let (result_tx, mut result_rx) = mpsc::channel::<ClassificationResult>(100);

    // Spawn worker thread
    let result_tx = result_tx.clone();
    tokio::spawn(async move {
        // let state = AppState::new();
        let classifier = Classifier::new(None);
        while let Some(task) = task_rx.recv().await {
            classifier.classify(&task.parsed_line, task.key);
            let success = true;
            let result = ClassificationResult {
                key: task.parsed_line.id.clone(),
                success,
            };
            if let Err(e) = result_tx.send(result).await {
                error!("Failed to send result to main thread: {}", e);
            }
        }
    });

    // Spawn a thread to process results
    tokio::spawn(async move {
        while let Some(result) = result_rx.recv().await {
            if result.success {
                // Commit the successfully processed task
                info!("Successfully processed task for key: {}", result.key);
            } else {
                error!("Failed to process task for key: {}", result.key);
            }
        }
    });

    // Main loop to process records and submit tasks
    while let Some(Ok(record)) = consumer.next().await {
        let payload = record.value().to_vec();
        let key = record.key().map(|k| k.to_vec());

        let data_str = String::from_utf8_lossy(&payload);
        let key_str = String::from_utf8_lossy(&key.unwrap_or_default()).to_string();

        match from_str::<ParsedLine>(&data_str) {
            Ok(parsed_line) => {
                let task = ClassificationTask {
                    parsed_line,
                    key: key_str,
                };
                if let Err(e) = task_tx.send(task).await {
                    error!("Failed to send task to worker: {}", e);
                }
            }
            Err(e) => error!("Failed to deserialize record: {}", e),
        }
    }

    Ok(())
}
