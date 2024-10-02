use crate::connections::shared::error::ConfigError;

use super::config::GreptimeConfig;
use greptimedb_ingester::{Client as GreptimeClient, ClientBuilder, Database, StreamInserter};
use rocket::{request::FromRequest, State};
use sqlx::Error as SqlxError;
use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};
use std::borrow::Cow;
use thiserror::Error;

#[derive(Clone)]
pub struct GreptimeConnection {
    pub client: GreptimeClient,
    pub admin_psql: Pool<Postgres>,
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
    #[error("GreptimeDB streaming inserter error: {0}")]
    StreamingInserterError(#[from] greptimedb_ingester::Error),
}

impl GreptimeConnection {
    pub async fn new() -> Result<Self, GreptimeConnectionError> {
        let config = GreptimeConfig::new()?;
        // GreptimeDB Client
        let client = ClientBuilder::default()
            .peers(vec![config.get_uri()])
            .build();

        // GreptimeDB PostgreSQL Admin
        let admin_psql = PgPoolOptions::new()
            .max_connections(1)
            .connect(&config.get_psql_uri("public"))
            .await?;

        Ok(Self {
            client,
            admin_psql,
            config,
        })
    }
    pub fn streaming_inserter(
        &self,
        customer_id: &str,
    ) -> Result<StreamInserter, GreptimeConnectionError> {
        let database = Database::new_with_dbname(customer_id, self.client.clone());
        database
            .streaming_inserter()
            .map_err(GreptimeConnectionError::from)
    }

    pub async fn create_database_if_not_exists(
        &self,
        customer_id: &str,
    ) -> Result<(), GreptimeConnectionError> {
        if !self.database_exists(customer_id).await? {
            self.create_database(customer_id).await?;
        }
        Ok(())
    }

    async fn database_exists(&self, customer_id: &str) -> Result<bool, GreptimeConnectionError> {
        let result = sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
            .bind(customer_id)
            .fetch_optional(&self.admin_psql)
            .await?;
        Ok(result.is_some())
    }

    async fn create_database(&self, customer_id: &str) -> Result<(), GreptimeConnectionError> {
        tracing::info!("Creating database {}.", customer_id);
        sqlx::query(&format!("CREATE DATABASE {}", customer_id))
            .execute(&self.admin_psql)
            .await
            .map_err(|e| match e {
                Error::Database(ref db_err) if db_err.code() == Some(Cow::Borrowed("22023")) => {
                    // this could happen if the database was created between the check and the create
                    tracing::info!("Database {} already exists.", customer_id);
                    Ok(())
                }
                _ => Err(GreptimeConnectionError::DbCreationError(
                    customer_id.to_owned(),
                    e,
                )),
            })
            .ok();
        Ok(())
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
