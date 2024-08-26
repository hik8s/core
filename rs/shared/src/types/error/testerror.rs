use crate::{
    connections::redis::connect::RedisConnectionError, types::classifier::error::ClassifierError,
};

use super::classificationerror::ClassificationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Classification error: {0}")]
    ClassificationError(#[from] ClassificationError),
    #[error("Classifier error: {0}")]
    ClassifierError(#[from] ClassifierError),
}
