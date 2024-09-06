use std::{
    env::{var, VarError},
    num::ParseFloatError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Error: {0}, Environment variable {1}")]
    EnvVarError(#[source] VarError, String),
    #[error("Error: {0}")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

pub fn get_env_var(key: &str) -> Result<String, ConfigError> {
    var(key).map_err(|e| ConfigError::EnvVarError(e, key.to_owned()))
}
