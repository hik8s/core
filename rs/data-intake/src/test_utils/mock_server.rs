use rocket::local::asynchronous::Client;
use shared::db::greptime::connect::GreptimeConnection;

use crate::utils::rocket::build_rocket::build_rocket;

pub async fn rocket_test_client() -> Client {
    dotenv::dotenv().ok();
    let connection = GreptimeConnection::new().await.unwrap();
    let rocket = build_rocket(connection);
    Client::untracked(rocket)
        .await
        .expect("Failed to create Rocket test instance")
}
