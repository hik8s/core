use fluvio::FluvioError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FluvioConnectionError {
    #[error("Fluvio error: {0}")]
    Fluvio(#[from] FluvioError),
    #[error("Rocket error: {0}")]
    Rocket(String),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("Consumer config error: {0}")]
    ConsumerConfigError(String),
    #[error("Consumer error: {0}")]
    ConsumerError(String),
}
