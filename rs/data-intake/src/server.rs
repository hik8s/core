use crate::error::DataIntakeError;
use crate::event::route::event_intake;
use crate::log::route::log_intake;
use rocket::{routes, Build, Rocket};
use shared::router::rocket::{build_rocket, Connection};
use shared::{
    connections::greptime::connect::GreptimeConnection,
    fluvio::{FluvioConnection, TopicName},
};

pub async fn initialize_data_intake() -> Result<Rocket<Build>, DataIntakeError> {
    std::env::set_var("ROCKET_ADDRESS", "0.0.0.0");
    let greptime = GreptimeConnection::new().await?;
    let fluvio = FluvioConnection::new(TopicName::Log).await?;
    let connections: Vec<Connection> = vec![greptime.into(), fluvio.into()];
    let server = build_rocket(&connections, routes![log_intake, event_intake]);
    Ok(server)
}
