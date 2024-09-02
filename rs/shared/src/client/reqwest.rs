use reqwest::{Client, Error};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReqwestError {
    #[error("HTTP client error: {0}")]
    ClientError(#[from] Error),
    #[error("Reqwest error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct ReqwestClient {
    // this is a wrapped reqwest client necessary to implement a rocket fairing
    inner: Arc<Client>,
}

impl ReqwestClient {
    pub fn new() -> Result<Self, ReqwestError> {
        let client = Client::builder().use_rustls_tls().build()?;
        let wrapped_client = ReqwestClient {
            inner: Arc::new(client),
        };
        Ok(wrapped_client)
    }

    pub fn inner(&self) -> &Client {
        &self.inner
    }
}

pub fn get_http_client() -> Result<ReqwestClient, ReqwestError> {
    Ok(ReqwestClient::new()?)
}
