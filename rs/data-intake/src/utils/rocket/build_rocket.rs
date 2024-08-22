use rocket::{catch, catchers, routes, Build, Rocket};
use shared::connections::{
    fluvio::connect::FluvioConnection, greptime::connect::GreptimeConnection,
};

use crate::route::log::log_intake;

pub fn build_rocket(
    greptime_connection: GreptimeConnection,
    fluvio_connection: FluvioConnection,
) -> Rocket<Build> {
    rocket::build()
        .attach(greptime_connection)
        .attach(fluvio_connection)
        .mount("/", routes![log_intake])
        .register("/", catchers![internal_error])
}

#[catch(500)]
fn internal_error() -> &'static str {
    // TODO: add a request id to trace errors
    "Internal Server Error. Please try again later."
}
