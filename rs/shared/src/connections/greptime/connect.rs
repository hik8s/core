use super::config::GreptimeConfig;
use greptimedb_ingester::{ClientBuilder, Database};
use rocket::{request::FromRequest, State};
use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};
use std::{borrow::Cow, process};
#[derive(Clone)]
pub struct GreptimeConnection {
    pub greptime: Database,
    pub psql: Pool<Postgres>,
    pub config: GreptimeConfig,
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
        let greptime = Database::new_with_dbname(config.db_name.clone(), grpc_client);

        // GreptimeDB PostgreSQL
        let admin_psql = PgPoolOptions::new()
            .max_connections(5)
            .connect(&format!(
                "{psql_uri}/public",
                psql_uri = config.get_psql_uri()
            ))
            .await
            .unwrap();

        sqlx::query(&format!("CREATE DATABASE {}", config.db_name))
            .execute(&admin_psql)
            .await
            .map_err(|e| match e {
                Error::Database(ref db_err) if db_err.code() == Some(Cow::Borrowed("22023")) => {
                    tracing::info!("Database {} already exists.", config.db_name);
                }
                _ => {
                    tracing::warn!("Failed to create database {}: {:?}", config.db_name, e);
                    process::exit(1);
                }
            })
            .ok();

        // Now connect to the newly created database
        let psql = PgPoolOptions::new()
            .max_connections(5)
            .connect(&format!(
                "{psql_uri}/{db_name}",
                psql_uri = config.get_psql_uri(),
                db_name = config.db_name
            ))
            .await
            .unwrap();

        Ok(Self {
            greptime,
            psql,
            config,
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
