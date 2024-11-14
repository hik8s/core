use fluvio::{metadata::topic::TopicSpec, FluvioAdmin};
use tracing::info;

use crate::{
    constant::{
        TOPIC_CLASS_BYTES_PER_RECORD, TOPIC_CLASS_NAME, TOPIC_CLASS_PARTITIONS,
        TOPIC_CLASS_REPLICAS, TOPIC_LOG_BYTES_PER_RECORD, TOPIC_LOG_NAME, TOPIC_LOG_PARTITIONS,
        TOPIC_LOG_REPLICAS,
    },
    log_error,
};

use super::error::FluvioConnectionError;

use strum::EnumIter;

#[derive(Debug, Clone, Copy, EnumIter, Hash, Eq, PartialEq)]
pub enum TopicName {
    Log,
    Class,
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
        .map_err(|e| FluvioConnectionError::AdminList(log_error!(e).into()))?;
    if topics.iter().any(|t| t.name == topic.name) {
        info!("Topic '{}' already exists", topic.name);
        return Ok(());
    }

    // Create the topic if it does not exist
    let topic_spec = TopicSpec::new_computed(topic.partitions, topic.replicas, None);
    admin
        .create(topic.name.to_owned(), false, topic_spec)
        .await
        .map_err(|e| FluvioConnectionError::AdminCreate(log_error!(e).into()))?;
    info!("Topic '{}' created", topic.name);
    Ok(())
}
