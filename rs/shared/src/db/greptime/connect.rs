use super::config::GreptimeConfig;
use greptimedb_ingester::{ClientBuilder, Database};
use rocket::{request::FromRequest, State};
use sqlx::mysql::MySqlPoolOptions;
use std::process;

#[derive(Clone)]
pub struct GreptimeConnection {
    pub greptime: Database,
    // pub greptime_sql: Pool<MySql>,
}

impl GreptimeConnection {
    pub async fn new() -> Result<Self, String> {
        let config = match GreptimeConfig::new() {
            Ok(config) => config,
            Err(e) => {
                tracing::error!("Error: {}. Failed to create DbConfig.", e);
                process::exit(1);
            }
        };
        // GreptimeDB
        let grpc_client = ClientBuilder::default()
            .peers(vec![config.get_uri()])
            .build();
        let greptime = Database::new_with_dbname(config.collection_name.clone(), grpc_client);

        // GreptimeDB MySQL
        let greptime_sql = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&config.get_mysql_uri())
            .await
            .unwrap();
        sqlx::query(&format!(
            "CREATE DATABASE IF NOT EXISTS {}",
            config.collection_name.clone()
        ))
        .execute(&greptime_sql)
        .await
        .unwrap();

        Ok(Self {
            greptime,
            // greptime_sql,
        })
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GreptimeConnection {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let connection = request.guard::<&State<GreptimeConnection>>().await.unwrap();
        rocket::request::Outcome::Success(connection.inner().clone())
    }
}

// Define a CRUD middleware
#[rocket::async_trait]
impl rocket::fairing::Fairing for GreptimeConnection {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "GreptimeDB Connection",
            kind: rocket::fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
        Ok(rocket.manage(self.clone()))
    }
}
