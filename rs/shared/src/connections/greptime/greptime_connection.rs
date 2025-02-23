use crate::log_error;
use crate::ConfigError;

use super::config::GreptimeConfig;
use greptimedb_ingester::{Client as GreptimeClient, ClientBuilder, Database, StreamInserter};
use rocket::{request::FromRequest, State};
use sqlx::Error as SqlxError;
use sqlx::{postgres::PgPoolOptions, Error, Executor, Pool, Postgres, Row};
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
    pub fn streaming_inserter(&self, key: &str) -> Result<StreamInserter, GreptimeConnectionError> {
        let database = Database::new_with_dbname(key, self.client.clone());
        database
            .streaming_inserter()
            .map_err(|e| log_error!(e).into())
    }

    pub async fn create_database(&self, key: &str) -> Result<(), GreptimeConnectionError> {
        let result = sqlx::query(&format!("CREATE DATABASE {}", key))
            .execute(&self.admin_psql)
            .await;
        if let Err(e) = result {
            match e {
                Error::Database(ref db_err) if db_err.code() == Some(Cow::Borrowed("22023")) => {
                    // this could happen if the database was created between the check and the create
                    tracing::debug!("Database {} already exists.", key);
                }
                e => return Err(log_error!(e).into()),
            }
        }
        Ok(())
    }
    pub async fn connect_db(&self, key: &str) -> Result<Pool<Postgres>, GreptimeConnectionError> {
        let psql_uri = self.config.get_psql_uri(key);
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&psql_uri)
            .await
            .map_err(|e| log_error!(e).into())
    }

    pub async fn rename_table(
        &self,
        key: &str,
        table_name: &str,
        new_table_name: &str,
    ) -> Result<(), sqlx::Error> {
        let psql = self
            .connect_db(key)
            .await
            .map_err(|e| log_error!(e))
            .unwrap();
        let query = format!(
            "ALTER TABLE \"{}\" RENAME \"{}\"",
            table_name, new_table_name
        );
        psql.execute(query.as_str()).await?;
        Ok(())
    }

    pub async fn list_tables(&self, key: &str) -> Result<Vec<String>, sqlx::Error> {
        let psql = self
            .connect_db(key)
            .await
            .map_err(|e| log_error!(e))
            .unwrap();
        let query = "SHOW TABLES";
        let rows = psql.fetch_all(query).await?;
        let tables = rows.iter().map(|row| row.get("Tables")).collect();
        Ok(tables)
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        connections::greptime::middleware::insert::{create_insert_request, create_string_columns},
        get_env_var, setup_tracing,
        utils::mock::mock_client::generate_podname,
        DbName,
    };

    #[tokio::test]
    async fn test_table_rename_delete() -> Result<(), sqlx::Error> {
        setup_tracing(true);

        // greptime connection
        let greptime = GreptimeConnection::new().await.unwrap();
        let db = DbName::Resource;
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let key = db.key(&customer_id);

        // test table
        let mut map = HashMap::<&str, String>::new();
        map.insert("apiVersion", "v1".to_string());
        map.insert("kind", "Pod".to_string());
        let columns = create_string_columns(map, Some(1620000000));

        let table_name = generate_podname("test-pod");
        let req = create_insert_request(&table_name, columns, 1);

        // insert data
        let inserter = greptime.streaming_inserter(&key).unwrap();
        inserter.insert(vec![req]).await.unwrap();
        inserter.finish().await.unwrap();

        // rename table
        let table_name_deleted = format!("{table_name}___deleted");
        greptime
            .rename_table(&key, &table_name, &table_name_deleted)
            .await
            .unwrap();
        let table_names = greptime.list_tables(&key).await?;

        // assert rename success
        assert!(
            !table_names.contains(&table_name),
            "Original table should not exist"
        );
        assert!(
            table_names.contains(&table_name_deleted),
            "Deleted table should exist"
        );
        Ok(())
    }
}
