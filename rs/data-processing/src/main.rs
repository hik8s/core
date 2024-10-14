use shared::constant::TOPIC_LOG_PARTITIONS;
use shared::fluvio::{FluvioConnectionError, TopicName};
use shared::{fluvio::FluvioConnection, tracing::setup::setup_tracing};
use thiserror::Error;
use threads::process::{process_logs, ProcessThreadError};
use tokio::task::JoinHandle;
use tracing::error;

pub mod threads;

#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Process thread error: {0}")]
    ProcessThreadError(#[from] ProcessThreadError),
}

pub async fn run_data_processing() -> Result<(), DataProcessingError> {
    let fluvio_logs = FluvioConnection::new(TopicName::Log).await?;
    let fluvio_class = FluvioConnection::new(TopicName::Class).await?;

    // Vector to hold all spawned threads
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();

    // Loop through each partition to create consumers and processing threads
    for partition_id in 0..fluvio_logs.topic.partitions {
        let log_consumer = fluvio_logs.create_consumer(partition_id).await?;
        let class_producer = fluvio_class.producer.clone();
        threads.push(tokio::spawn(async move {
            process_logs(log_consumer, class_producer).await?;
            Ok(())
        }));
    }

    // Wait for all threads to complete
    for thread in threads {
        thread.await??;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing();
    run_data_processing().await?;
    Ok(())
}
