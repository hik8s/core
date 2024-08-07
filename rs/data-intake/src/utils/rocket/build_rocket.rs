use rocket::{routes, Build, Rocket};
use shared::db::greptime::connect::GreptimeConnection;

use crate::routes::logs::log_intake;

pub fn build_rocket(connection: GreptimeConnection) -> Rocket<Build> {
    rocket::build()
        .attach(connection)
        .mount("/", routes![log_intake])
}
