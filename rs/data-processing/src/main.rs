use shared::connections::fluvio::connect::{ConnectionError, BATCH_SIZE, PARTITIONS};
use shared::{connections::fluvio::connect::FluvioConnection, tracing::setup::setup_tracing};
use thiserror::Error;
use threads::consume::consume_logs;
use threads::process::process_logs;
use threads::types::communication::{ClassificationResult, ClassificationTask};
use tokio::sync::mpsc;
use tracing::error;

pub mod threads;

#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] ConnectionError),
    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Other error: {0}")]
    Other(#[from] Box<dyn std::error::Error>),
}

#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing();
    let fluvio_connection = FluvioConnection::new().await?;

    // Vector to hold all spawned threads
    let mut threads = Vec::new();

    // Loop through each partition to create consumers and processing threads
    for partition_id in 0..PARTITIONS {
        let consumer = fluvio_connection.create_consumer(partition_id).await?;
        let (result_sender, result_receiver) = mpsc::channel::<ClassificationResult>(BATCH_SIZE);
        let (data_sender, data_receiver) = mpsc::channel::<ClassificationTask>(BATCH_SIZE);

        // Spawn worker thread for each partition
        threads.push(tokio::spawn(async move {
            process_logs(data_receiver, result_sender).await;
        }));

        // Spawn a thread to consume logs for each partition
        threads.push(tokio::spawn(async move {
            consume_logs(consumer, data_sender, result_receiver).await;
        }));
    }

    // Wait for all threads to complete
    for thread in threads {
        thread.await.map_err(DataProcessingError::from)?;
    }

    Ok(())
}
