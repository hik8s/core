use crate::connections::{
    fluvio::connect::FluvioConnection, greptime::connect::GreptimeConnection,
};
use rocket::{catch, catchers, Build, Rocket, Route};

pub fn build_rocket_data_intake(
    greptime_connection: GreptimeConnection,
    fluvio_connection: FluvioConnection,
    routes: Vec<Route>,
) -> Rocket<Build> {
    rocket::build()
        .attach(greptime_connection)
        .attach(fluvio_connection)
        .mount("/", routes)
        .register("/", catchers![internal_error])
}

#[catch(500)]
fn internal_error() -> &'static str {
    // TODO: add a request id to trace errors
    "Internal Server Error. Please try again later."
}
