const DEFAULT_GREPTIME_RPC_PORT: &str = "4001";
const DEFAULT_GREPTIME_PSQL_PORT: &str = "4003";

use crate::{get_env_var, ConfigError};

#[derive(Clone)]
pub struct GreptimeConfig {
    pub host: String,
    pub port: String,
    pub psql_port: String,
}
impl GreptimeConfig {
    pub fn new() -> Result<Self, ConfigError> {
        Ok(Self {
            host: get_env_var("GREPTIMEDB_HOST")?,
            port: DEFAULT_GREPTIME_RPC_PORT.to_owned(),
            psql_port: DEFAULT_GREPTIME_PSQL_PORT.to_owned(),
        })
    }
    pub fn get_uri(&self) -> String {
        let GreptimeConfig { host, port, .. } = self;
        format!("{host}:{port}")
    }
    pub fn get_psql_uri(&self, key: &str) -> String {
        let GreptimeConfig {
            host, psql_port, ..
        } = self;
        format!("postgresql://{host}:{psql_port}/{key}")
    }
}
