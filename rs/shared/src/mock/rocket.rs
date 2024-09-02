use crate::{
    connections::{
        greptime::connect::GreptimeConnectionError, redis::connect::RedisConnectionError,
    },
    fluvio::FluvioConnectionError,
    router::rocket::{build_rocket, Attach},
};
use rocket::{local::asynchronous::Client, Route};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TestClientError {
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Rocket error: {0}")]
    RocketError(#[from] rocket::Error),
}

pub async fn rocket_test_client(
    connections: &[impl Attach],
    routes: Vec<Route>,
) -> Result<Client, TestClientError> {
    dotenv::dotenv().ok();
    let rocket = build_rocket(connections, routes);
    let client = Client::untracked(rocket).await?;
    Ok(client)
}
