use std::{env::VarError, num::ParseFloatError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Error: {0}, Environment variable {1}")]
    EnvVarError(#[source] VarError, String),
    #[error("Error: {0}")]
    ParseFloatError(#[from] ParseFloatError),
}
