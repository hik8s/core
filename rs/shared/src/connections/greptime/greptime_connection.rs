use crate::log_error;
use crate::ConfigError;
use crate::DbName;

use super::config::GreptimeConfig;
use greptimedb_ingester::{Client as GreptimeClient, ClientBuilder, Database, StreamInserter};
use rocket::{request::FromRequest, State};
use sqlx::Error as SqlxError;
use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};
use std::borrow::Cow;
use thiserror::Error;
use tracing::error;

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
        db: &DbName,
        customer_id: &str,
    ) -> Result<StreamInserter, GreptimeConnectionError> {
        let database = Database::new_with_dbname(db.id(customer_id), self.client.clone());
        database
            .streaming_inserter()
            .map_err(|e| log_error!(e).into())
    }

    pub async fn create_database(
        &self,
        db: &DbName,
        customer_id: &str,
    ) -> Result<(), GreptimeConnectionError> {
        let db_id = db.id(customer_id);
        let result = sqlx::query(&format!("CREATE DATABASE {}", db_id))
            .execute(&self.admin_psql)
            .await;
        if let Err(e) = result {
            match e {
                Error::Database(ref db_err) if db_err.code() == Some(Cow::Borrowed("22023")) => {
                    // this could happen if the database was created between the check and the create
                    tracing::debug!("Database {} already exists.", db_id);
                }
                e => return Err(log_error!(e).into()),
            }
        }
        Ok(())
    }
    pub async fn connect_db(
        &self,
        db: &DbName,
        customer_id: &str,
    ) -> Result<Pool<Postgres>, GreptimeConnectionError> {
        let psql_uri = self.config.get_psql_uri(&db.id(customer_id));
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&psql_uri)
            .await
            .map_err(|e| log_error!(e).into())
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
