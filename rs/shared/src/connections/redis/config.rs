use std::env::var;

use crate::connections::shared::error::ConfigError;

pub struct RedisConfig {
    pub host: String,
}
impl RedisConfig {
    pub fn new() -> Result<Self, ConfigError> {
        Ok(Self {
            host: var("REDIS_HOST")
                .map_err(|e| ConfigError::EnvVarError(e, "REDIS_HOST".to_owned()))?,
        })
    }
    pub fn get_uri(&self) -> String {
        format!("redis://{}", self.host)
    }
}
