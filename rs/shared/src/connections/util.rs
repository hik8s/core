use tracing::warn;

use crate::ConfigError;
use std::env::var;

pub fn get_env_var(key: &str) -> Result<String, ConfigError> {
    var(key).map_err(|e| ConfigError::EnvVarError(e, key.to_owned()))
}

pub fn get_env_var_as_vec(key: &str) -> Result<Option<Vec<String>>, ConfigError> {
    let vec: Vec<String> = var(key)
        .map_err(|e| ConfigError::EnvVarError(e, key.to_owned()))?
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    if vec.is_empty() {
        warn!("Empty env var: {}", key);
        return Ok(None);
    }
    Ok(Some(vec))
}

#[cfg(test)]
mod tests {
    use crate::setup_tracing;

    use super::*;
    use std::env;

    #[test]
    fn test_get_env_var_as_vec() {
        setup_tracing(false);
        // Empty case
        env::set_var("TEST_KEY", "");
        assert_eq!(get_env_var_as_vec("TEST_KEY").unwrap(), None);

        // Single value
        env::set_var("TEST_KEY", "value1");
        assert_eq!(
            get_env_var_as_vec("TEST_KEY").unwrap(),
            Some(vec!["value1".to_string()])
        );

        // Multiple values
        env::set_var("TEST_KEY", "value1,value2");
        assert_eq!(
            get_env_var_as_vec("TEST_KEY").unwrap(),
            Some(vec!["value1".to_string(), "value2".to_string()])
        );

        // Missing env var
        env::remove_var("TEST_KEY");
        assert!(get_env_var_as_vec("TEST_KEY").is_err());
    }
}
