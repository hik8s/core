use shared::{
    connections::greptime::connect::GreptimeConnectionError, fluvio::FluvioConnectionError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataIntakeError {
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Rocket error: {0}")]
    RocketError(#[from] rocket::Error),
}
