use logs::types::appstate::AppStateError;
use shared::connections::fluvio::connect::ConnectionError;
use shared::{connections::fluvio::connect::FluvioConnection, tracing::setup::setup_tracing};
use thiserror::Error;
use threads::commit::commit_results;
use threads::consume::consume_logs;
use threads::process::process_logs;
use threads::types::communication::{ClassificationResult, ClassificationTask};
use tokio::sync::mpsc;
use tracing::error;

pub mod logs;
pub mod threads;

#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] ConnectionError),
    #[error("Other error: {0}")]
    Other(#[from] Box<dyn std::error::Error>),
    #[error("App state error: {0}")]
    AppStateError(#[from] AppStateError),
}

#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing();

    let fluvio_connection = FluvioConnection::new().await?;
    let consumer = fluvio_connection.create_consumer().await?;

    // Create channels for task submission and result collection
    let (data_sender, data_receiver) = mpsc::channel::<ClassificationTask>(100);
    let (result_sender, result_receiver) = mpsc::channel::<ClassificationResult>(100);

    // Spawn worker thread
    tokio::spawn(process_logs(data_receiver, result_sender.clone()));

    // Spawn a thread to process results
    tokio::spawn(commit_results(result_receiver));

    // Main loop to process records and submit tasks
    consume_logs(consumer, data_sender).await;
    Ok(())
}
