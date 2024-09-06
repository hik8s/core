use crate::{connections::ConfigError, get_env_var};

const DEFAULT_QDRANT_PORT: &str = "6334";

#[derive(Clone)]
pub struct QdrantConfig {
    pub host: String,
    pub port: String,
    pub collection_name: String,
}
impl QdrantConfig {
    pub fn new(collection_name: String) -> Result<Self, ConfigError> {
        let host = get_env_var("QDRANT_HOST")?;

        Ok(Self {
            host,
            port: DEFAULT_QDRANT_PORT.to_owned(),
            collection_name,
        })
    }
    pub fn get_qdrant_uri(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}
