use fluvio::dataplane::record::ConsumerRecord;
use shared::constant::FLUVIO_BATCH_SIZE;
use shared::fluvio::{FluvioConnectionError, TopicName};
use shared::{fluvio::FluvioConnection, tracing::setup::setup_tracing};
use thiserror::Error;
use threads::consume::{consume_logs, ConsumerThreadError};
use threads::process::{process_logs, ProcessThreadError};
use tokio::sync::mpsc;
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
    #[error("Consumer thread error: {0}")]
    ConsumerThreadError(#[from] ConsumerThreadError),
}

#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing();
    let fluvio_connection_log = FluvioConnection::new(TopicName::Log).await?;
    let fluvio_connection_class = FluvioConnection::new(TopicName::Class).await?;
    // Vector to hold all spawned threads
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();

    // Loop through each partition to create consumers and processing threads
    for partition_id in 0..fluvio_connection_log.topic.partitions {
        let consumer = fluvio_connection_log.create_consumer(partition_id).await?;
        let (result_sender, result_receiver) = mpsc::channel::<(String, String)>(FLUVIO_BATCH_SIZE);
        let (data_sender, data_receiver) = mpsc::channel::<ConsumerRecord>(FLUVIO_BATCH_SIZE);

        // Spawn worker thread for each partition
        let fluvio_connection_class_clone = fluvio_connection_class.clone();
        threads.push(tokio::spawn(async move {
            process_logs(data_receiver, result_sender, fluvio_connection_class_clone).await?;
            Ok(())
        }));

        // Spawn a thread to consume logs for each partition
        threads.push(tokio::spawn(async move {
            consume_logs(consumer, data_sender, result_receiver).await?;
            Ok(())
        }));
    }

    // Wait for all threads to complete
    for thread in threads {
        thread.await??;
    }

    Ok(())
}
