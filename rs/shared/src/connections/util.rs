use std::env::var;

use crate::constant::LOG_PREFIX;

use super::ConfigError;

pub fn get_env_var(key: &str) -> Result<String, ConfigError> {
    var(key).map_err(|e| ConfigError::EnvVarError(e, key.to_owned()))
}
