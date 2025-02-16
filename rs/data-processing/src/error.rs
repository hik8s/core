use crate::threads::error::ProcessThreadError;
use shared::FluvioConnectionError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Process thread error: {0}")]
    ProcessThreadError(#[from] ProcessThreadError),
}
