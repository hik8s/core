use crate::{get_env_var, ConfigError};

pub struct RedisConfig {
    pub host: String,
    password: String,
}
impl RedisConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let host = get_env_var("REDIS_HOST")?;
        let password = get_env_var("REDIS_PASSWORD")?;
        Ok(Self { host, password })
    }
    pub fn get_uri(&self) -> String {
        format!("redis://:{}@{}", self.password, self.host)
    }
}
