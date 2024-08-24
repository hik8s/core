use rocket::local::asynchronous::Client;
use shared::connections::{
    fluvio::connect::{ConnectionError, FluvioConnection, TOPIC_NAME_LOG},
    greptime::connect::{GreptimeConnection, GreptimeConnectionError},
    redis::connect::RedisConnectionError,
};
use thiserror::Error;

use crate::utils::rocket::build::build_rocket;

#[derive(Error, Debug)]
pub enum TestClientError {
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] ConnectionError),
}

pub async fn rocket_test_client() -> Result<Client, TestClientError> {
    dotenv::dotenv().ok();
    let greptime_connection = GreptimeConnection::new().await?;
    let fluvio_connection = FluvioConnection::new(&TOPIC_NAME_LOG.to_owned()).await?;
    let rocket = build_rocket(greptime_connection, fluvio_connection);
    let client = Client::untracked(rocket)
        .await
        .expect("Failed to create Rocket test instance");
    Ok(client)
}
