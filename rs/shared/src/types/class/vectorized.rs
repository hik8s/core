use std::collections::HashMap;

use crate::{constant::EMBEDDING_USIZE, types::tokenizer::Tokenizer};
use qdrant_client::qdrant::{PointStruct, ScoredPoint, Value};
use serde::{Deserialize, Serialize};
use serde_json::Error as JsonError;
use tracing::debug;

use super::Class;

pub trait Id {
    fn get_id(&self) -> &str;
    fn get_data(&self) -> &str;
}

impl Id for VectorizedClass {
    fn get_id(&self) -> &str {
        &self.class_id
    }
    fn get_data(&self) -> &str {
        &self.representation
    }
}

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

pub fn to_qdrant_point<T: Serialize + Id>(
    item: &T,
    array: [f32; EMBEDDING_USIZE],
) -> Result<PointStruct, JsonError> {
    let payload = serde_json::to_string(&item)?;
    let payload: HashMap<String, Value> = serde_json::from_str(&payload)?;
    let point = PointStruct::new(item.get_id(), array.to_vec(), payload);
    Ok(point)
}

pub fn to_qdrant_points<T: Serialize + Id>(
    data: &[T],
    arrays: &[[f32; EMBEDDING_USIZE]],
) -> Result<Vec<PointStruct>, JsonError> {
    data.iter()
        .zip(arrays.iter())
        .map(|(item, array)| to_qdrant_point(item, *array))
        .collect()
}

pub fn from_scored_point(
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

pub fn to_vectorized_classes(
    classes: &[Class],
    tokenizer: &Tokenizer,
) -> (Vec<VectorizedClass>, usize) {
    classes.iter().map(|class| class.vectorize(tokenizer)).fold(
        (Vec::new(), 0),
        |(mut vec, sum), vc| {
            let token_count_cut = vc.token_count_cut;
            debug!(
                "Vectorized class: {} with {} tokens",
                vc.key, vc.token_count_cut
            );
            vec.push(vc);
            (vec, sum + token_count_cut as usize)
        },
    )
}

pub fn vectorize(classes: &[Class], tokenizer: &Tokenizer) -> (Vec<VectorizedClass>, usize) {
    to_vectorized_classes(classes, tokenizer)
}

pub fn to_representations(vectorized_classes: &[VectorizedClass]) -> Vec<String> {
    vectorized_classes
        .iter()
        .map(|vc| vc.representation.clone())
        .collect()
}
