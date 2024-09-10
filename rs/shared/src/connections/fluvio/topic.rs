use fluvio::{metadata::topic::TopicSpec, FluvioAdmin};
use tracing::info;

use crate::constant::{
    TOPIC_CLASS_NAME, TOPIC_CLASS_PARTITIONS, TOPIC_CLASS_REPLICAS, TOPIC_LOG_NAME,
    TOPIC_LOG_PARTITIONS, TOPIC_LOG_REPLICAS,
};

use super::error::FluvioConnectionError;

pub enum TopicName {
    Log,
    Class,
}

#[derive(Clone)]
pub struct FluvioTopic {
    pub name: String,
    pub partitions: u32,
    pub replicas: u32,
}

impl FluvioTopic {
    pub fn new(topic: TopicName) -> Self {
        match topic {
            TopicName::Log => FluvioTopic {
                name: TOPIC_LOG_NAME.to_owned(),
                partitions: TOPIC_LOG_PARTITIONS,
                replicas: TOPIC_LOG_REPLICAS,
            },
            TopicName::Class => FluvioTopic {
                name: TOPIC_CLASS_NAME.to_owned(),
                partitions: TOPIC_CLASS_PARTITIONS,
                replicas: TOPIC_CLASS_REPLICAS,
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
        .map_err(|e| FluvioConnectionError::AdminList(e.into()))?;
    if topics.iter().any(|t| t.name == topic.name) {
        info!("Topic '{}' already exists", topic.name);
        return Ok(());
    }

    // Create the topic if it does not exist
    let topic_spec = TopicSpec::new_computed(topic.partitions, topic.replicas, None);
    admin
        .create(topic.name.to_owned(), false, topic_spec)
        .await
        .map_err(|e| FluvioConnectionError::AdminCreate(e.into()))?;
    info!("Topic '{}' created", topic.name);
    Ok(())
}
