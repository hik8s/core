use rocket::{local::asynchronous::Client, routes};
use shared::{
    connections::{
        fluvio::connect::{FluvioConnection, FluvioConnectionError, TopicName},
        greptime::connect::{GreptimeConnection, GreptimeConnectionError},
        redis::connect::RedisConnectionError,
    },
    router::rocket::build_rocket_data_intake,
};
use thiserror::Error;

use crate::route::log::log_intake;

#[derive(Error, Debug)]
pub enum TestClientError {
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
}

pub async fn rocket_test_client() -> Result<Client, TestClientError> {
    dotenv::dotenv().ok();
    let greptime_connection = GreptimeConnection::new().await?;
    let fluvio_connection = FluvioConnection::new(TopicName::Log).await?;
    let rocket =
        build_rocket_data_intake(greptime_connection, fluvio_connection, routes![log_intake]);
    let client = Client::untracked(rocket)
        .await
        .expect("Failed to create Rocket test instance");
    Ok(client)
}
