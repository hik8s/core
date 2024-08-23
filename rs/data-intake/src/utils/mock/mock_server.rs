use rocket::local::asynchronous::Client;
use shared::connections::{
    fluvio::connect::FluvioConnection, greptime::connect::GreptimeConnection,
};

use crate::utils::rocket::build::build_rocket;

pub async fn rocket_test_client() -> Client {
    dotenv::dotenv().ok();
    let greptime_connection = GreptimeConnection::new().await.unwrap();
    let fluvio_connection = FluvioConnection::new().await.unwrap();
    let rocket = build_rocket(greptime_connection, fluvio_connection);
    Client::untracked(rocket)
        .await
        .expect("Failed to create Rocket test instance")
}
