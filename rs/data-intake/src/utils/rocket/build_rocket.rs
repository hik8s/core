use rocket::{catch, catchers, routes, Build, Rocket};
use shared::db::greptime::connect::GreptimeConnection;

use crate::routes::logs::log_intake;

pub fn build_rocket(connection: GreptimeConnection) -> Rocket<Build> {
    rocket::build()
        .attach(connection)
        .mount("/", routes![log_intake])
        .register("/", catchers![internal_error])
}

#[catch(500)]
fn internal_error() -> &'static str {
    // TODO: add a request id to trace errors
    "Internal Server Error. Please try again later."
}
