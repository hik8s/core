use std::env::var;

use crate::connections::shared::error::ConfigError;

const DEFAULT_QDRANT_PORT: &str = "6334";

#[derive(Clone)]
pub struct QdrantConfig {
    pub host: String,
    pub port: String,
    pub collection_name: String,
}
impl QdrantConfig {
    pub fn new(collection_name: String) -> Result<Self, ConfigError> {
        let host = var("QDRANT_HOST")
            .map_err(|e| ConfigError::EnvVarError(e, "QDRANT_HOST".to_owned()))?;

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
