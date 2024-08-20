pub mod middleware;
pub mod process;
pub mod route;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

use rocket::main;

use shared::{
    connections::{fluvio::connect::FluvioConnection, greptime::connect::GreptimeConnection},
    tracing::setup::setup_tracing,
};
use utils::rocket::build_rocket::build_rocket;

#[main]
async fn main() -> () {
    std::env::set_var("ROCKET_ADDRESS", "0.0.0.0");
    setup_tracing();

    let greptime_connection = GreptimeConnection::new().await.unwrap();
    let fluvio_connection = FluvioConnection::new().await.unwrap();

    let rocket = build_rocket(greptime_connection, fluvio_connection);

    match rocket.launch().await {
        Ok(_) => (),
        Err(error) => {
            tracing::error!("Failed to launch Rocket: {}", error);
        }
    }
}
