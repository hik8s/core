use crate::error::DataProcessingError;
use crate::threads::process_event::process_event;
use crate::threads::process_log::process_logs;
use crate::threads::process_resource::process_resource;

use shared::connections::dbname::DbName;
use shared::connections::fluvio::topic::FluvioTopic;
use shared::fluvio::TopicName;
use shared::log_error_with_message;
use shared::FluvioConnection;
use tokio::task::JoinHandle;

pub fn run_log_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();

    // Loop through each partition to create consumers and processing threads
    for partition_id in 0..FluvioTopic::new(TopicName::Log).partitions {
        threads.push(tokio::spawn(async move {
            let fluvio = FluvioConnection::new().await?;
            let log_consumer = fluvio.create_consumer(partition_id, TopicName::Log).await?;
            let class_producer = fluvio.get_producer(TopicName::Class).clone();
            process_logs(log_consumer, class_producer)
                .await
                .map_err(|e| {
                    log_error_with_message!("Log processing thread exited with error", e)
                })?;
            Ok(())
        }));
    }
    Ok(threads)
}

pub fn run_resource_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        let fluvio = FluvioConnection::new().await?;
        let consumer = fluvio.create_consumer(0, TopicName::Resource).await?;
        let producer = fluvio.get_producer(TopicName::ProcessedResource).clone();
        process_resource(consumer, producer, DbName::Resource)
            .await
            .map_err(|e| {
                log_error_with_message!("Resource processing thread exited with error", e)
            })?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_customresource_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        let fluvio = FluvioConnection::new().await?;
        let consumer = fluvio.create_consumer(0, TopicName::CustomResource).await?;
        let producer = fluvio
            .get_producer(TopicName::ProcessedCustomResource)
            .clone();
        process_resource(consumer, producer, DbName::CustomResource)
            .await
            .map_err(|e| {
                log_error_with_message!("Custom resource processing thread exited with error", e)
            })?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_event_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        let fluvio = FluvioConnection::new().await?;
        let consumer = fluvio.create_consumer(0, TopicName::Event).await?;
        let producer = fluvio.get_producer(TopicName::ProcessedEvent).clone();
        process_event(consumer, producer, DbName::Event)
            .await
            .map_err(|e| log_error_with_message!("Event processing thread exited with error", e))?;
        Ok(())
    }));

    Ok(threads)
}
