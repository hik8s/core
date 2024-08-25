use crate::connections::shared::error::ConfigError;

use super::config::GreptimeConfig;
use greptimedb_ingester::{ClientBuilder, Database, Result as GreptimeResult, StreamInserter};
use rocket::{request::FromRequest, State};
use sqlx::Error as SqlxError;
use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};
use std::borrow::Cow;
use thiserror::Error;

#[derive(Clone)]
pub struct GreptimeConnection {
    pub greptime: Database,
    pub psql: Pool<Postgres>,
    pub config: GreptimeConfig,
}

#[derive(Error, Debug)]
pub enum GreptimeConnectionError {
    #[error("Failed to create DbConfig: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Failed to build gRPC client: {0}")]
    GrpcClientError(String),
    #[error("Failed to connect to PostgreSQL: {0}")]
    PostgresConnectionError(#[from] SqlxError),
    #[error("PostgreSQL failed to create database: '{0}', error: {1}")]
    DbCreationError(String, #[source] SqlxError),
}

impl GreptimeConnection {
    pub async fn new() -> Result<Self, GreptimeConnectionError> {
        let config = GreptimeConfig::new()?;
        // GreptimeDB
        let grpc_client = ClientBuilder::default()
            .peers(vec![config.get_uri()])
            .build();
        let greptime = Database::new_with_dbname(config.db_name.clone(), grpc_client);

        // GreptimeDB PostgreSQL
        let admin_psql = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.get_psql_uri("public"))
            .await?;

        sqlx::query(&format!("CREATE DATABASE {}", config.db_name))
            .execute(&admin_psql)
            .await
            .map_err(|e| match e {
                Error::Database(ref db_err) if db_err.code() == Some(Cow::Borrowed("22023")) => {
                    tracing::info!("Database {} already exists.", config.db_name);
                    Ok(())
                }
                _ => Err(GreptimeConnectionError::DbCreationError(
                    config.db_name.to_owned(),
                    e,
                )),
            })
            .ok();

        // Now connect to the newly created database
        let psql = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.get_psql_uri(&config.db_name))
            .await?;

        Ok(Self {
            greptime,
            psql,
            config,
        })
    }
    pub fn streaming_inserter(&self) -> GreptimeResult<StreamInserter> {
        self.greptime.streaming_inserter()
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
