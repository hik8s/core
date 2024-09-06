use reqwest::{Client, Error as ClientError};
use std::sync::Arc;

use thiserror::Error;

use crate::connections::ConfigError;
use crate::constant::{PROMPT_ENGINE_PATH, PROMPT_ENGINE_PORT};
use crate::get_env_var;

#[derive(Error, Debug)]
pub enum PromptEngineError {
    #[error("HTTP client error: {0}")]
    ClientError(#[from] ClientError),
    #[error("Reqwest error: {0}")]
    AnyhowError(#[from] anyhow::Error),
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
}

#[derive(Clone)]
pub struct PromptEngineConfig {
    pub host: String,
    pub port: String,
    pub path: String,
}
impl PromptEngineConfig {
    pub fn new() -> Result<Self, ConfigError> {
        Ok(Self {
            host: get_env_var("PROMPT_ENGINE_HOST")?,
            port: PROMPT_ENGINE_PORT.to_owned(),
            path: PROMPT_ENGINE_PATH.to_owned(),
        })
    }
    pub fn get_uri(&self) -> String {
        format!("http://{}:{}/{}", self.host, self.port, self.path)
    }
}

#[derive(Clone)]
pub struct PromptEngineConnection {
    pub client: Arc<Client>,
    pub config: PromptEngineConfig,
}

impl PromptEngineConnection {
    pub fn new() -> Result<Self, PromptEngineError> {
        let config = PromptEngineConfig::new()?;
        let client = Client::builder().use_rustls_tls().build()?;
        let connection = PromptEngineConnection {
            client: Arc::new(client),
            config,
        };
        Ok(connection)
    }
    pub async fn request_augmentation(
        &self,
        user_message: String,
    ) -> Result<String, PromptEngineError> {
        let endpoint = self.config.get_uri();
        let response = self.client.post(endpoint).body(user_message).send().await?;
        Ok(response.text().await?)
    }
}
