const DEFAULT_GREPTIME_RPC_PORT: &str = "4001";
const DEFAULT_GREPTIME_PSQL_PORT: &str = "4003";

use std::env::var;

#[derive(Clone)]
pub struct GreptimeConfig {
    pub host: String,
    pub port: String,
    pub psql_port: String,
    pub db_name: String,
}
impl GreptimeConfig {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            host: var("GREPTIMEDB_HOST")
                .map_err(|e| format!("{e}: GREPTIMEDB_HOST: provide localhost or FQDN"))?,
            port: DEFAULT_GREPTIME_RPC_PORT.to_owned(),
            psql_port: DEFAULT_GREPTIME_PSQL_PORT.to_owned(),
            db_name: var("DB_NAME").map_err(|e| format!("{e}: DB_NAME"))?,
        })
    }
    pub fn get_uri(&self) -> String {
        let GreptimeConfig { host, port, .. } = self;
        format!("{host}:{port}")
    }
    pub fn get_psql_uri(&self) -> String {
        let GreptimeConfig {
            host, psql_port, ..
        } = self;
        format!("postgresql://{host}:{psql_port}")
    }
}
