const DEFAULT_GREPTIME_RPC_PORT: &str = "4001";
const DEFAULT_GREPTIME_MYSQL_PORT: &str = "4002";

pub struct GreptimeConfig {
    pub host: String,
    pub port: String,
    pub sql_port: String,
    pub collection_name: String,
}
impl GreptimeConfig {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            host: std::env::var("GREPTIMEDB_HOST")
                .map_err(|e| format!("{e}: GREPTIMEDB_HOST: provide localhost or FQDN"))?,
            port: DEFAULT_GREPTIME_RPC_PORT.to_owned(),
            sql_port: DEFAULT_GREPTIME_MYSQL_PORT.to_owned(),
            collection_name: std::env::var("COLLECTION_NAME")
                .map_err(|e| format!("{e}: COLLECTION_NAME"))?,
        })
    }
    pub fn get_uri(&self) -> String {
        let GreptimeConfig { host, port, .. } = self;
        format!("{host}:{port}")
    }
    pub fn get_mysql_uri(&self) -> String {
        let GreptimeConfig { host, sql_port, .. } = self;
        format!("mysql://{host}:{sql_port}")
    }
}
