pub mod log;
pub mod process;

use rocket::{main, routes, Error};

use log::route::log_intake;
use shared::{
    connections::{
        fluvio::{FluvioConnection, TopicName},
        greptime::connect::GreptimeConnection,
    },
    router::rocket::{build_rocket, Connection},
    tracing::setup::setup_tracing,
};

#[main]
async fn main() -> Result<(), Error> {
    std::env::set_var("ROCKET_ADDRESS", "0.0.0.0");
    setup_tracing();

    let greptime = GreptimeConnection::new().await.unwrap();
    let fluvio = FluvioConnection::new(TopicName::Log).await.unwrap();

    let connections: Vec<Connection> = vec![greptime.into(), fluvio.into()];
    let rocket = build_rocket(&connections, routes![log_intake]);

    rocket.launch().await?;
    Ok(())
}
