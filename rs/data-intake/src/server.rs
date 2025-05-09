use crate::error::DataIntakeError;
use crate::route::{
    customresource_intake, customresources_intake, event_intake, events_intake, log_intake,
    resource_intake, resources_intake,
};
use rocket::{routes, Build, Rocket};
use shared::router::rocket::{build_rocket, Connection};
use shared::{FluvioConnection, GreptimeConnection};

pub async fn initialize_data_intake() -> Result<Rocket<Build>, DataIntakeError> {
    let greptime = GreptimeConnection::new().await?;
    let fluvio = FluvioConnection::new().await?;

    let connections: Vec<Connection> = vec![greptime.into(), fluvio.into()];
    let routes = routes![
        log_intake,
        event_intake,
        events_intake,
        resource_intake,
        resources_intake,
        customresource_intake,
        customresources_intake
    ];

    let server = build_rocket(&connections, routes);
    Ok(server)
}
