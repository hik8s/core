use std::collections::HashMap;

use qdrant_client::qdrant::{PointStruct, Value};
use serde::{Deserialize, Serialize};
use serde_json::Error as JsonError;

use crate::constant::EMBEDDING_USIZE;

use super::class::Class;

#[derive(Serialize, Deserialize)]
pub struct QdrantPoint {
    pub class_id: String,
    pub container: String,
    pub key: String,
    pub length: usize,
    pub namespace: String,
    pub representation: String,
    pub token_count_original: u32,
    pub token_count_cut: u32,
}

impl QdrantPoint {
    pub fn new(class: Class, token_count_cut: u32, representation: String) -> Self {
        Self {
            class_id: class.class_id,
            container: class.container,
            key: class.key,
            length: class.length,
            namespace: class.namespace,
            representation,
            token_count_original: class.token_count,
            token_count_cut: token_count_cut,
        }
    }
}

pub fn to_qdrant_point(
    class: Class,
    token_count: u32,
    representation: String,
    vector: [f32; EMBEDDING_USIZE],
) -> Result<PointStruct, JsonError> {
    let qdrant_point = QdrantPoint::new(class, token_count, representation);
    let payload = serde_json::to_string(&qdrant_point)?;
    let payload: HashMap<String, Value> = serde_json::from_str(&payload)?;
    let point = PointStruct::new(qdrant_point.class_id, vector.to_vec(), payload);
    Ok(point)
}
