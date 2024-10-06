use reqwest::{Client, Error as ClientError};
use std::sync::Arc;

use thiserror::Error;

use crate::connections::ConfigError;
use crate::constant::{PROMPT_ENGINE_PATH, PROMPT_ENGINE_PORT};
use crate::{get_env_var, log_error};

#[derive(Error, Debug)]
pub enum PromptEngineError {
    #[error("HTTP client error: {0}")]
    ClientError(#[from] ClientError),
    #[error("Reqwest error: {0}")]
    AnyhowError(#[from] anyhow::Error),
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
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

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct AugmentationRequest {
    pub user_message: String,
    pub client_id: String,
}

impl AugmentationRequest {
    pub fn new(user_message: &str, client_id: &str) -> Self {
        AugmentationRequest {
            user_message: user_message.to_owned(),
            client_id: client_id.to_owned(),
        }
    }
}
impl TryInto<String> for AugmentationRequest {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        serde_json::to_string(&self).map_err(|e| log_error!(e))
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
        request: AugmentationRequest,
    ) -> Result<String, PromptEngineError> {
        let endpoint = self.config.get_uri();
        let body: String = request.try_into()?;
        let response = self.client.post(endpoint).body(body).send().await?;
        Ok(response.text().await?)
    }
}
