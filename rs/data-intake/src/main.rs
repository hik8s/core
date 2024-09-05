pub mod log;
pub mod process;

use rocket::{main, routes};
use thiserror::Error;

use log::route::log_intake;
use shared::{
    connections::greptime::connect::{GreptimeConnection, GreptimeConnectionError},
    fluvio::{FluvioConnection, FluvioConnectionError, TopicName},
    router::rocket::{build_rocket, Connection},
    tracing::setup::setup_tracing,
};

#[derive(Error, Debug)]
pub enum DataIntakeError {
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Rocket error: {0}")]
    RocketError(#[from] rocket::Error),
}

#[main]
async fn main() -> Result<(), DataIntakeError> {
    std::env::set_var("ROCKET_ADDRESS", "0.0.0.0");
    setup_tracing();

    let greptime = GreptimeConnection::new().await?;
    let fluvio = FluvioConnection::new(TopicName::Log).await?;

    let connections: Vec<Connection> = vec![greptime.into(), fluvio.into()];
    let rocket = build_rocket(&connections, routes![log_intake]);

    rocket.launch().await?;
    Ok(())
}
