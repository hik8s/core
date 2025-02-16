use std::collections::HashMap;

use qdrant_client::qdrant;
use qdrant_client::qdrant::ScoredPoint;
use serde::{Deserialize, Serialize};
use uuid7::uuid4;

use crate::types::class::vectorized::Id;

pub mod config;
pub mod error;
pub mod qdrant_connection;
mod test_qdrant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQdrantMetadata {
    pub kind: String,
    pub qdrant_uid: String,
    pub resource_uid: String,
    pub name: String,
    pub namespace: String,
    pub data: String,
    pub data_type: String,
}
impl ResourceQdrantMetadata {
    pub fn new(
        kind: String,
        resource_uid: String,
        name: String,
        namespace: String,
        data: String,
        data_type: String,
    ) -> Self {
        Self {
            kind,
            qdrant_uid: uuid4().to_string(),
            resource_uid,
            name,
            namespace,
            data,
            data_type,
        }
    }
}

impl Id for ResourceQdrantMetadata {
    fn get_id(&self) -> &str {
        &self.qdrant_uid
    }
    fn get_data(&self) -> &str {
        &self.data
    }
}

impl TryFrom<ScoredPoint> for ResourceQdrantMetadata {
    type Error = serde_json::Error;

    fn try_from(point: ScoredPoint) -> Result<Self, Self::Error> {
        let payload: HashMap<String, qdrant::Value> = point.payload;
        let json_value = serde_json::to_value(payload)?;
        let resource: ResourceQdrantMetadata = serde_json::from_value(json_value)?;
        Ok(resource)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventQdrantMetadata {
    pub apiversion: String,
    pub kind: String,
    pub qdrant_uid: String,
    pub resource_uid: String,
    pub name: String,
    pub namespace: String,
    pub message: String,
    pub reason: String,
    pub event_type: String,
    pub data: String,
}

impl EventQdrantMetadata {
    pub fn new(
        apiversion: String,
        kind: String,
        resource_uid: String,
        name: String,
        namespace: String,
        message: String,
        reason: String,
        event_type: String,
        data: String,
    ) -> Self {
        Self {
            apiversion,
            kind,
            qdrant_uid: uuid4().to_string(),
            resource_uid,
            name,
            namespace,
            message,
            reason,
            event_type,
            data,
        }
    }
}

impl TryFrom<ScoredPoint> for EventQdrantMetadata {
    type Error = serde_json::Error;

    fn try_from(point: ScoredPoint) -> Result<Self, Self::Error> {
        let payload: HashMap<String, qdrant::Value> = point.payload;
        let json_value = serde_json::to_value(payload)?;
        let event: EventQdrantMetadata = serde_json::from_value(json_value)?;
        Ok(event)
    }
}

impl Id for EventQdrantMetadata {
    fn get_id(&self) -> &str {
        &self.qdrant_uid
    }
    fn get_data(&self) -> &str {
        &self.data
    }
}
