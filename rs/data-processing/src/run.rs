use crate::error::DataProcessingError;
use crate::threads::process::process_logs;
use crate::threads::process_event::process_event;
use crate::threads::process_resource::process_resource;

use shared::connections::dbname::DbName;
use shared::fluvio::FluvioConnection;
use shared::fluvio::TopicName;
use tokio::task::JoinHandle;

pub async fn run_data_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let fluvio = FluvioConnection::new().await?;

    // Vector to hold all spawned threads
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();

    // Loop through each partition to create consumers and processing threads
    for partition_id in 0..fluvio.get_topic(TopicName::Log).partitions {
        let log_consumer = fluvio.create_consumer(partition_id, TopicName::Log).await?;
        let class_producer = fluvio.get_producer(TopicName::Class).clone();
        threads.push(tokio::spawn(async move {
            process_logs(log_consumer, class_producer).await?;
            Ok(())
        }));
    }
    Ok(threads)
}

pub async fn run_resource_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let fluvio = FluvioConnection::new().await?;
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();
    threads.push(tokio::spawn(async move {
        let resource_consumer = fluvio.create_consumer(0, TopicName::Resource).await?;
        process_resource(resource_consumer, DbName::Resource).await?;
        Ok(())
    }));

    Ok(threads)
}

pub async fn run_customresource_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let fluvio = FluvioConnection::new().await?;
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();
    threads.push(tokio::spawn(async move {
        let resource_consumer = fluvio.create_consumer(0, TopicName::CustomResource).await?;
        process_resource(resource_consumer, DbName::CustomResource).await?;
        Ok(())
    }));

    Ok(threads)
}

pub async fn run_event_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let fluvio = FluvioConnection::new().await?;
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();
    threads.push(tokio::spawn(async move {
        let resource_consumer = fluvio.create_consumer(0, TopicName::Event).await?;
        process_event(resource_consumer, DbName::Event).await?;
        Ok(())
    }));

    Ok(threads)
}
