use fluvio::{metadata::topic::TopicSpec, FluvioAdmin};
use tracing::info;

use crate::{
    constant::{
        TOPIC_CLASS_BYTES_PER_RECORD, TOPIC_CLASS_NAME, TOPIC_CLASS_PARTITIONS,
        TOPIC_CLASS_REPLICAS, TOPIC_CUSTOM_RESOURCE_BYTES_PER_RECORD, TOPIC_CUSTOM_RESOURCE_NAME,
        TOPIC_CUSTOM_RESOURCE_PARTITIONS, TOPIC_CUSTOM_RESOURCE_REPLICAS,
        TOPIC_EVENT_BYTES_PER_RECORD, TOPIC_EVENT_NAME, TOPIC_EVENT_PARTITIONS,
        TOPIC_EVENT_REPLICAS, TOPIC_LOG_BYTES_PER_RECORD, TOPIC_LOG_NAME, TOPIC_LOG_PARTITIONS,
        TOPIC_LOG_REPLICAS, TOPIC_PROCESSED_CUSTOM_RESOURCE_NAME, TOPIC_PROCESSED_EVENT_NAME,
        TOPIC_PROCESSED_RESOURCE_NAME, TOPIC_RESOURCE_BYTES_PER_RECORD, TOPIC_RESOURCE_NAME,
        TOPIC_RESOURCE_PARTITIONS, TOPIC_RESOURCE_REPLICAS,
    },
    log_error,
};

use super::error::FluvioConnectionError;

use strum::EnumIter;

#[derive(Debug, Clone, Copy, EnumIter, Hash, Eq, PartialEq)]
pub enum TopicName {
    Log,
    Event,
    Resource,
    CustomResource,
    Class,
    ProcessedEvent,
    ProcessedResource,
    ProcessedCustomResource,
}

#[derive(Clone)]
pub struct FluvioTopic {
    pub name: String,
    pub partitions: u32,
    pub replicas: u32,
    pub max_bytes: usize,
}

impl FluvioTopic {
    pub fn new(topic: TopicName) -> Self {
        match topic {
            TopicName::Log => FluvioTopic {
                name: TOPIC_LOG_NAME.to_owned(),
                partitions: TOPIC_LOG_PARTITIONS,
                replicas: TOPIC_LOG_REPLICAS,
                max_bytes: TOPIC_LOG_BYTES_PER_RECORD,
            },
            TopicName::Class => FluvioTopic {
                name: TOPIC_CLASS_NAME.to_owned(),
                partitions: TOPIC_CLASS_PARTITIONS,
                replicas: TOPIC_CLASS_REPLICAS,
                max_bytes: TOPIC_CLASS_BYTES_PER_RECORD,
            },
            TopicName::Event => FluvioTopic {
                name: TOPIC_EVENT_NAME.to_owned(),
                partitions: TOPIC_EVENT_PARTITIONS,
                replicas: TOPIC_EVENT_REPLICAS,
                max_bytes: TOPIC_EVENT_BYTES_PER_RECORD,
            },
            TopicName::Resource => FluvioTopic {
                name: TOPIC_RESOURCE_NAME.to_owned(),
                partitions: TOPIC_RESOURCE_PARTITIONS,
                replicas: TOPIC_RESOURCE_REPLICAS,
                max_bytes: TOPIC_RESOURCE_BYTES_PER_RECORD,
            },
            TopicName::CustomResource => FluvioTopic {
                name: TOPIC_CUSTOM_RESOURCE_NAME.to_owned(),
                partitions: TOPIC_CUSTOM_RESOURCE_PARTITIONS,
                replicas: TOPIC_CUSTOM_RESOURCE_REPLICAS,
                max_bytes: TOPIC_CUSTOM_RESOURCE_BYTES_PER_RECORD,
            },
            TopicName::ProcessedEvent => FluvioTopic {
                name: TOPIC_PROCESSED_EVENT_NAME.to_owned(),
                partitions: TOPIC_EVENT_PARTITIONS,
                replicas: TOPIC_EVENT_REPLICAS,
                max_bytes: TOPIC_EVENT_BYTES_PER_RECORD,
            },
            TopicName::ProcessedResource => FluvioTopic {
                name: TOPIC_PROCESSED_RESOURCE_NAME.to_owned(),
                partitions: TOPIC_RESOURCE_PARTITIONS,
                replicas: TOPIC_RESOURCE_REPLICAS,
                max_bytes: TOPIC_RESOURCE_BYTES_PER_RECORD,
            },
            TopicName::ProcessedCustomResource => FluvioTopic {
                name: TOPIC_PROCESSED_CUSTOM_RESOURCE_NAME.to_owned(),
                partitions: TOPIC_RESOURCE_PARTITIONS,
                replicas: TOPIC_RESOURCE_REPLICAS,
                max_bytes: TOPIC_RESOURCE_BYTES_PER_RECORD,
            },
        }
    }
}

pub async fn create_topic(
    admin: FluvioAdmin,
    topic: &FluvioTopic,
) -> Result<(), FluvioConnectionError> {
    let topics = admin
        .list::<TopicSpec, String>(vec![])
        .await
        .map_err(|e| FluvioConnectionError::AdminList(log_error!(e)))?;
    if topics.iter().any(|t| t.name == topic.name) {
        info!("Topic '{}' already exists", topic.name);
        return Ok(());
    }

    // Create the topic if it does not exist
    let topic_spec = TopicSpec::new_computed(topic.partitions, topic.replicas, None);
    admin
        .create(topic.name.to_owned(), false, topic_spec)
        .await
        .map_err(|e| FluvioConnectionError::AdminCreate(log_error!(e)))?;
    info!("Topic '{}' created", topic.name);
    Ok(())
}
