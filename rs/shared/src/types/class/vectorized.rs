use std::collections::HashMap;

use crate::constant::EMBEDDING_USIZE;
use qdrant_client::qdrant::{PointStruct, ScoredPoint, Value};
use serde::{Deserialize, Serialize};
use serde_json::Error as JsonError;

use super::Class;

#[derive(Serialize, Deserialize, Debug)]
pub struct VectorizedClass {
    pub class_id: String,
    pub container: String,
    pub key: String,
    pub length: usize,
    pub namespace: String,
    pub representation: String,
    pub token_count_original: u32,
    pub token_count_cut: u32,
    pub score: f32,
}

impl VectorizedClass {
    pub fn new(class: Class, token_count_cut: usize, representation: String) -> Self {
        Self {
            class_id: class.class_id,
            container: class.container,
            key: class.key,
            length: class.length,
            namespace: class.namespace,
            representation,
            token_count_original: class.token_count,
            token_count_cut: token_count_cut as u32,
            score: 0.0,
        }
    }
}

impl TryFrom<ScoredPoint> for VectorizedClass {
    type Error = serde_json::Error;

    fn try_from(point: ScoredPoint) -> Result<Self, Self::Error> {
        let payload: HashMap<String, Value> = point.payload;
        let json_value = serde_json::to_value(payload)?;
        let mut vectorized_class: VectorizedClass = serde_json::from_value(json_value)?;
        vectorized_class.score = point.score;
        Ok(vectorized_class)
    }
}

pub fn to_qdrant_point(
    class: VectorizedClass,
    array: [f32; EMBEDDING_USIZE],
) -> Result<PointStruct, JsonError> {
    let payload = serde_json::to_string(&class)?;
    let payload: HashMap<String, Value> = serde_json::from_str(&payload)?;
    let point = PointStruct::new(class.class_id, array.to_vec(), payload);
    Ok(point)
}

pub fn to_vectorized_class(
    points: Vec<ScoredPoint>,
) -> Result<Vec<VectorizedClass>, serde_json::Error> {
    points
        .iter()
        .map(|point_struct| {
            let mut vc = VectorizedClass::try_from(point_struct.to_owned())
                .map_err(serde_json::Error::from)?;
            vc.score = point_struct.score;
            Ok(vc)
        })
        .collect()
}
