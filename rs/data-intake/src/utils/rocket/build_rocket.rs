use rocket::{routes, Build, Rocket};
use shared::db::greptime::connect::GreptimeConnection;

use crate::routes::logline::logline;

pub fn build_rocket(connection: GreptimeConnection) -> Rocket<Build> {
    rocket::build()
        .attach(connection)
        .mount("/", routes![logline])
}
