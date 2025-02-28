use crate::constant::GREPTIME_TABLE_KEY;
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
    pub fn streaming_inserter(&self, db: &str) -> Result<StreamInserter, GreptimeConnectionError> {
        let database = Database::new_with_dbname(db, self.client.clone());
        database
            .streaming_inserter()
            .map_err(|e| log_error!(e).into())
    }

    pub async fn create_database(&self, db: &str) -> Result<(), GreptimeConnectionError> {
        let result = sqlx::query(&format!("CREATE DATABASE {}", db))
            .execute(&self.admin_psql)
            .await;
        if let Err(e) = result {
            match e {
                Error::Database(ref db_err) if db_err.code() == Some(Cow::Borrowed("22023")) => {
                    // this could happen if the database was created between the check and the create
                    tracing::debug!("Database {} already exists.", db);
                }
                e => return Err(log_error!(e).into()),
            }
        }
        Ok(())
    }
    pub async fn connect_db(&self, db: &str) -> Result<Pool<Postgres>, GreptimeConnectionError> {
        let psql_uri = self.config.get_psql_uri(db);
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&psql_uri)
            .await
            .map_err(|e| log_error!(e).into())
    }

    pub async fn rename_table(
        &self,
        db: &str,
        table_name: &str,
        new_table_name: &str,
    ) -> Result<(), sqlx::Error> {
        let psql = self
            .connect_db(db)
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

    pub async fn mark_table_deleted(&self, db: &str, table_name: &str) -> Result<(), sqlx::Error> {
        let table_name_deleted = format!("{table_name}___deleted");
        self.rename_table(db, table_name, &table_name_deleted).await
    }

    pub async fn list_tables(
        &self,
        db: &str,
        filter: Option<&str>,
        resource_filter: Option<&str>,
        exclude_deleted: bool,
    ) -> Result<Vec<String>, sqlx::Error> {
        // TODO: handle error gracefully
        let psql = match self.connect_db(db).await.map_err(|e| log_error!(e)) {
            Ok(psql) => psql,
            Err(e) => {
                error!("Failed to connect to PostgreSQL: {}", e);
                return Ok(vec![]);
            }
        };

        let key = GREPTIME_TABLE_KEY;
        let mut conditions = Vec::new();

        // Add filter condition if provided
        if let Some(filter) = filter {
            conditions.push(format!("{key} LIKE '%{filter}%'"));
        }

        // Add resource_filter condition if provided
        if let Some(resource_filter) = resource_filter {
            conditions.push(format!("{key} LIKE '{resource_filter}__%'"));
        }

        // Add exclude_deleted condition if needed
        if exclude_deleted {
            conditions.push(format!("{key} NOT LIKE '%___deleted'"));
        }

        // Build the query
        let query = if conditions.is_empty() {
            "SHOW TABLES".to_string()
        } else {
            format!("SHOW TABLES WHERE {}", conditions.join(" and "))
        };

        // Execute query and return results
        let rows = psql.fetch_all(query.as_str()).await?;
        let tables = rows.iter().map(|row| row.get(GREPTIME_TABLE_KEY)).collect();
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

#[derive(Debug)]
pub struct GreptimeTable {
    pub kind: String,
    pub namespace: String,
    pub name: String,
    pub uid: String,
    pub is_deleted: bool,
}

pub fn parse_resource_name(resource_name: &str) -> Option<GreptimeTable> {
    let parts: Vec<&str> = resource_name.split("__").collect();

    if parts.len() >= 4 {
        let (uid, is_deleted) = if let Some((uid, _)) = parts[3].split_once("___deleted") {
            (uid.to_string(), true)
        } else {
            (parts[3].to_string(), false)
        };

        Some(GreptimeTable {
            kind: parts[0].to_string(),
            namespace: parts[1].to_string(),
            name: parts[2].to_string(),
            uid,
            is_deleted,
        })
    } else {
        None
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

    #[test]
    fn test_parse_resource_name() {
        setup_tracing(false);
        let table_name =
        "certificate__examples__hello-server-hik9s__692cf3ae-680c-4b00-949d-e26dbf781a40___deleted";
        let parsed = parse_resource_name(table_name).unwrap();
        tracing::debug!("{:?}", parsed);
        assert_eq!(parsed.kind, "certificate");
        assert_eq!(parsed.namespace, "examples");
        assert_eq!(parsed.name, "hello-server-hik9s");
        assert_eq!(parsed.uid, "692cf3ae-680c-4b00-949d-e26dbf781a40");
        assert!(parsed.is_deleted, "Table should be marked as deleted");

        // Test a non-deleted table
        let non_deleted = "pod__default__nginx__abcd1234";
        let parsed = parse_resource_name(non_deleted).unwrap();
        assert!(!parsed.is_deleted, "Table should not be marked as deleted");
    }

    #[tokio::test]
    async fn test_table_rename_delete() -> Result<(), sqlx::Error> {
        setup_tracing(true);

        // greptime connection
        let greptime = GreptimeConnection::new().await.unwrap();
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let db = DbName::Resource.id(&customer_id);

        // test table
        let mut map = HashMap::<&str, String>::new();
        map.insert("apiVersion", "v1".to_string());
        map.insert("kind", "Pod".to_string());
        let columns = create_string_columns(map, Some(1620000000));

        let table_name = generate_podname("test-pod");
        let req = create_insert_request(&table_name, columns, 1);

        // insert data
        let inserter = greptime.streaming_inserter(&db).unwrap();
        inserter.insert(vec![req]).await.unwrap();
        inserter.finish().await.unwrap();

        // rename table
        greptime.mark_table_deleted(&db, &table_name).await.unwrap();
        let table_names = greptime.list_tables(&db, None, None, false).await?;

        // assert rename success
        assert!(
            !table_names.contains(&table_name),
            "Original table should not exist"
        );
        assert!(
            table_names.contains(&format!("{table_name}___deleted")),
            "Deleted table should exist"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_filter_deleted_tables() -> Result<(), sqlx::Error> {
        setup_tracing(true);

        // Setup
        let greptime = GreptimeConnection::new().await.unwrap();
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let db = DbName::Resource.id(&customer_id);

        // Query all tables and active tables
        let all_tables = greptime.list_tables(&db, None, None, false).await?;
        let active_tables = greptime.list_tables(&db, None, None, true).await?;

        // Check if delete filter filters all deleted tables
        let active_tables_filtered_len = active_tables
            .iter()
            .filter_map(|name| parse_resource_name(name))
            .filter(|i| !i.is_deleted)
            .count();
        assert_eq!(
            active_tables.len(),
            active_tables_filtered_len,
            "Expect to active tables to filter all deleted tables"
        );
        assert!(
            all_tables.len() > active_tables.len(),
            "Total number of tables should be larger than active tables"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_tables() -> Result<(), sqlx::Error> {
        setup_tracing(true);

        // greptime connection
        let greptime = GreptimeConnection::new().await.unwrap();
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let db = DbName::Resource.id(&customer_id);
        let table_names: Vec<String> = greptime.list_tables(&db, None, None, false).await?;
        let mut parsed_resources = table_names
            .iter()
            .filter_map(|name| parse_resource_name(name))
            .collect::<Vec<GreptimeTable>>();

        let total = parsed_resources.len();
        parsed_resources.retain(|r| !r.is_deleted);
        let active = parsed_resources.len();

        assert!(!parsed_resources.is_empty());
        tracing::debug!("Total resources: {}, Active resources: {}", total, active);
        tracing::debug!(
            "Active resources: {:?}",
            parsed_resources
                .iter()
                .map(|i| i.name.clone())
                .collect::<Vec<String>>()
        );

        Ok(())
    }
}
