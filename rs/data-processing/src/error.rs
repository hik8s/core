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
    #[error("Log processing thread exited with error: {0}")]
    LogProcessingExit(#[source] ProcessThreadError),
    #[error("Resource processing thread exited with error: {0}")]
    ResourceProcessingExit(#[source] ProcessThreadError),
    #[error("Custom resource processing thread exited with error: {0}")]
    CustomResourceProcessingExit(#[source] ProcessThreadError),
    #[error("Event processing thread exited with error: {0}")]
    EventProcessingExit(#[source] ProcessThreadError),
}
