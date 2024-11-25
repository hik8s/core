use crate::error::DataProcessingError;
use crate::threads::process::process_logs;
use crate::threads::process_event::process_event;
use crate::threads::process_resource::process_resource;

use shared::connections::dbname::DbName;
use shared::connections::fluvio::topic::FluvioTopic;
use shared::fluvio::FluvioConnection;
use shared::fluvio::TopicName;
use tokio::task::JoinHandle;

pub fn run_data_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    // Vector to hold all spawned threads
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();

    // Loop through each partition to create consumers and processing threads
    for partition_id in 0..FluvioTopic::new(TopicName::Log).partitions {
        threads.push(tokio::spawn(async move {
            let fluvio = FluvioConnection::new().await?;
            let log_consumer = fluvio.create_consumer(partition_id, TopicName::Log).await?;
            let class_producer = fluvio.get_producer(TopicName::Class).clone();
            process_logs(log_consumer, class_producer).await?;
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
        let resource_consumer = fluvio.create_consumer(0, TopicName::Resource).await?;
        process_resource(resource_consumer, DbName::Resource).await?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_customresource_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();
    threads.push(tokio::spawn(async move {
        let fluvio = FluvioConnection::new().await?;
        let resource_consumer = fluvio.create_consumer(0, TopicName::CustomResource).await?;
        process_resource(resource_consumer, DbName::CustomResource).await?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_event_processing(
) -> Result<Vec<JoinHandle<Result<(), DataProcessingError>>>, DataProcessingError> {
    let mut threads: Vec<JoinHandle<Result<(), DataProcessingError>>> = Vec::new();
    threads.push(tokio::spawn(async move {
        let fluvio = FluvioConnection::new().await?;
        let resource_consumer = fluvio.create_consumer(0, TopicName::Event).await?;
        process_event(resource_consumer, DbName::Event).await?;
        Ok(())
    }));

    Ok(threads)
}
