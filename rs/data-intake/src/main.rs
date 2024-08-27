pub mod process;
pub mod route;
pub mod utils;

use rocket::{main, routes, Error};

use route::log::log_intake;
use shared::{
    connections::{
        fluvio::connect::{FluvioConnection, TopicName},
        greptime::connect::GreptimeConnection,
    },
    router::rocket::build_rocket_data_intake,
    tracing::setup::setup_tracing,
};

#[main]
async fn main() -> Result<(), Error> {
    std::env::set_var("ROCKET_ADDRESS", "0.0.0.0");
    setup_tracing();

    let greptime_connection = GreptimeConnection::new().await.unwrap();
    let fluvio_connection = FluvioConnection::new(TopicName::Log).await.unwrap();

    let rocket =
        build_rocket_data_intake(greptime_connection, fluvio_connection, routes![log_intake]);

    rocket.launch().await?;
    Ok(())
}
