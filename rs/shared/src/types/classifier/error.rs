use std::num::ParseFloatError;

use thiserror::Error;

use crate::{
    connections::shared::error::ConfigError, types::error::classificationerror::ClassificationError,
};

#[derive(Error, Debug)]
pub enum ClassifierError {
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Classification error: {0}")]
    ClassificationError(#[from] ClassificationError),
    #[error("Tokenization error: {0}")]
    TokenizerError(#[from] TokenizerError),
    #[error("Error: {0}")]
    ParseFloatError(#[from] ParseFloatError),
}

#[derive(Error, Debug)]
pub enum TokenizerError {
    #[error("Tokenization error: {0}")]
    TokenizerError(#[from] anyhow::Error),
}
