use super::ConfigError;
use std::env::var;

pub fn get_env_var(key: &str) -> Result<String, ConfigError> {
    var(key).map_err(|e| ConfigError::EnvVarError(e, key.to_owned()))
}
