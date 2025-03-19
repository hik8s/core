use crate::constant::GREPTIME_TABLE_KEY;
use crate::log_error;
use crate::ConfigError;

use super::config::GreptimeConfig;
use greptimedb_ingester::{Client as GreptimeClient, ClientBuilder, Database, StreamInserter};
use rocket::{request::FromRequest, State};
use sqlx::postgres::PgRow;
use sqlx::Error as SqlxError;
use sqlx::{postgres::PgPoolOptions, Error, Executor, Pool, Postgres, Row};
use std::borrow::Cow;
use std::fmt;
use thiserror::Error;
use tracing::error;
use tracing::warn;

#[derive(Clone)]
pub struct GreptimeConnection {
    pub client: GreptimeClient,
    pub admin_psql: Pool<Postgres>,
    pub config: GreptimeConfig,
}

#[derive(Error, Debug)]
pub enum GreptimeConnectionError {
    #[error("Failed to create DbConfig: {0}")]
    Config(#[from] ConfigError),
    #[error("Failed to build gRPC client: {0}")]
    GrpcClient(String),
    #[error("Failed to connect to PostgreSQL: {0}")]
    PostgresConnection(#[from] SqlxError),
    #[error("GreptimeDB streaming inserter error: {0}")]
    StreamingInserter(#[from] greptimedb_ingester::Error),
    #[error("Failed to parse GreptimeTable from table name: {0}")]
    TableParse(String),
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
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&psql_uri)
            .await?;
        Ok(pool)
    }

    pub async fn rename_table(
        &self,
        db: &str,
        table_name: &str,
        new_table_name: &str,
    ) -> Result<(), GreptimeConnectionError> {
        let psql = self.connect_db(db).await?;
        let query = format!(
            "ALTER TABLE \"{}\" RENAME \"{}\"",
            table_name, new_table_name
        );
        psql.execute(query.as_str()).await?;
        Ok(())
    }

    pub async fn mark_table_deleted(
        &self,
        db: &str,
        mut table: GreptimeTable,
    ) -> Result<(), GreptimeConnectionError> {
        let table_name = table.format_name();
        table.is_deleted = true;
        let new_table_name = table.format_name();
        self.rename_table(db, &table_name, &new_table_name).await
    }

    pub async fn list_tables(
        &self,
        db: &str,
        general_filter: Option<&str>,
        resource_filter: Option<&str>,
        exclude_deleted: bool,
    ) -> Result<Vec<GreptimeTable>, GreptimeConnectionError> {
        // TODO: handle error gracefully
        let psql = self.connect_db(db).await?;

        // Add filter condition if provided
        let mut conditions = Vec::new();
        if let Some(filter) = general_filter {
            conditions.push(format!("{GREPTIME_TABLE_KEY} LIKE '%{filter}%'"));
        }

        // Add resource_filter condition if provided
        if let Some(filter) = resource_filter {
            conditions.push(format!("{GREPTIME_TABLE_KEY} LIKE '{filter}__%'"));
        }

        // Add exclude_deleted condition if needed
        if exclude_deleted {
            conditions.push(format!("{GREPTIME_TABLE_KEY} NOT LIKE '%___deleted'"));
        }

        // Build the query
        let query = if conditions.is_empty() {
            "SHOW TABLES".to_string()
        } else {
            format!("SHOW TABLES WHERE {}", conditions.join(" and "))
        };

        // Execute query and return results
        let rows = psql.fetch_all(query.as_str()).await?;
        let tables = rows
            .iter()
            .map(|row| row.get::<String, _>(GREPTIME_TABLE_KEY))
            .filter_map(|name| {
                GreptimeTable::try_from(&name)
                    .inspect_err(|err| warn!("Failed to parse table name '{}': {}", name, err))
                    .ok()
            })
            .collect();
        Ok(tables)
    }

    pub async fn query(
        &self,
        db: &str,
        table: &str,
        key: &str,
    ) -> Result<Vec<PgRow>, GreptimeConnectionError> {
        let psql = self.connect_db(db).await?;
        let query = format!("SELECT {key} FROM \"{table}\"");
        let rows = psql.fetch_all(&*query).await?;
        Ok(rows)
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

#[derive(Debug, PartialEq, Clone)]
pub struct GreptimeTable {
    pub kind: String,
    pub namespace: String,
    pub name: String,
    pub uid: String,
    pub is_deleted: bool,
}

impl GreptimeTable {
    pub fn new(
        kind: impl Into<String>,
        namespace: impl Into<String>,
        name: impl Into<String>,
        uid: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            namespace: namespace.into(),
            name: name.into(),
            uid: uid.into(),
            is_deleted: false,
        }
    }
    pub fn new_test_table(case: impl fmt::Display, kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            namespace: "test-namespace".to_string(),
            name: case.to_string(),
            uid: uuid7::uuid4().to_string(),
            is_deleted: false,
        }
    }

    pub fn print_table(&self) -> String {
        format!(
            "{} {} {}",
            self.namespace,
            self.name,
            if self.is_deleted { " (deleted)" } else { "" }
        )
    }
    pub fn format_name(&self) -> String {
        let base_name = format!(
            "{}__{}__{}__{}",
            self.kind, self.namespace, self.name, self.uid
        );
        if self.is_deleted {
            format!("{base_name}___deleted")
        } else {
            base_name
        }
    }
}
impl TryFrom<&str> for GreptimeTable {
    type Error = GreptimeConnectionError;

    fn try_from(table_name: &str) -> Result<Self, Self::Error> {
        // First check if the entire name contains the deleted suffix
        let is_deleted = table_name.contains("___deleted");

        // Split by double underscore
        let parts: Vec<&str> = table_name.split("__").collect();

        if parts.len() >= 4 {
            // Handle the UID part which might contain the deleted suffix
            let uid_part = parts[3];
            let uid = match uid_part.split_once("___deleted") {
                Some((uid, _)) => uid.to_string(),
                None => uid_part.to_string(),
            };

            Ok(GreptimeTable {
                kind: parts[0].to_string(),
                namespace: parts[1].to_string(),
                name: parts[2].to_string(),
                uid,
                is_deleted,
            })
        } else {
            Err(GreptimeConnectionError::TableParse(table_name.to_string()))
        }
    }
}
impl TryFrom<&String> for GreptimeTable {
    type Error = GreptimeConnectionError;

    fn try_from(table_name: &String) -> Result<Self, Self::Error> {
        // Simply delegate to the &str implementation
        GreptimeTable::try_from(table_name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tracing::warn;

    use super::*;
    use crate::{
        connections::greptime::middleware::insert::{create_insert_request, create_string_columns},
        get_env_var, setup_tracing, DbName,
    };

    #[test]
    fn test_parse_resource_name() {
        setup_tracing(false);
        let table_name =
        "certificate__examples__hello-server-hik9s__692cf3ae-680c-4b00-949d-e26dbf781a40___deleted";
        let parsed = GreptimeTable::try_from(table_name).unwrap();
        tracing::debug!("{:?}", parsed);
        assert_eq!(parsed.kind, "certificate");
        assert_eq!(parsed.namespace, "examples");
        assert_eq!(parsed.name, "hello-server-hik9s");
        assert_eq!(parsed.uid, "692cf3ae-680c-4b00-949d-e26dbf781a40");
        assert!(parsed.is_deleted, "Table should be marked as deleted");

        // Test a non-deleted table
        let non_deleted = "pod__default__nginx__abcd1234";
        let parsed = GreptimeTable::try_from(non_deleted).unwrap();
        assert!(!parsed.is_deleted, "Table should not be marked as deleted");
    }

    #[tokio::test]
    async fn test_table_rename_delete() -> Result<(), GreptimeConnectionError> {
        setup_tracing(true);

        // greptime connection
        let greptime = GreptimeConnection::new().await?;
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let db = DbName::Resource.id(&customer_id);

        // test table
        let mut map = HashMap::<&str, String>::new();
        map.insert("apiVersion", "v1".to_string());
        map.insert("kind", "Pod".to_string());
        let columns = create_string_columns(map, Some(1620000000));

        let table = GreptimeTable::new_test_table("test-pod", "pod");
        let req = create_insert_request(&table, columns, 1);

        // insert data
        greptime.create_database(&db).await?;
        let inserter = greptime.streaming_inserter(&db)?;
        inserter.insert(vec![req]).await?;
        inserter.finish().await?;

        // rename table
        greptime.mark_table_deleted(&db, table.clone()).await?;
        let table_names = greptime.list_tables(&db, None, None, false).await?;

        // assert rename success
        assert!(
            !table_names.contains(&table),
            "Original table should not exist"
        );
        let mut deleted_table = table.clone();
        deleted_table.is_deleted = true;
        assert!(
            table_names.contains(&deleted_table),
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
        let all_tables = greptime
            .list_tables(&db, None, None, false)
            .await
            .unwrap_or_default();
        let active_tables = greptime
            .list_tables(&db, None, None, true)
            .await
            .unwrap_or_default();

        if all_tables.is_empty() || active_tables.is_empty() {
            warn!("No tables found");
            // todo: handle better by e.g. inserting something
            return Ok(());
        }
        // Check if delete filter filters all deleted tables
        let active_tables_filtered_len = active_tables.iter().filter(|i| !i.is_deleted).count();
        assert_eq!(
            active_tables.len(),
            active_tables_filtered_len,
            "Expect to active tables to filter all deleted tables"
        );
        assert!(
            all_tables.len() >= active_tables.len(),
            "Total number of tables should be larger than active tables"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_tables() -> Result<(), GreptimeConnectionError> {
        setup_tracing(true);

        // greptime connection
        let greptime = GreptimeConnection::new().await?;
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let db = DbName::Resource.id(&customer_id);
        let mut tables = greptime
            .list_tables(&db, None, None, false)
            .await
            .unwrap_or_default();

        let total = tables.len();
        tables.retain(|r| !r.is_deleted);
        let active = tables.len();

        if tables.is_empty() {
            warn!("No active resources found");
        }
        tracing::debug!("Total resources: {}, Active resources: {}", total, active);
        tracing::debug!(
            "Active resources: {:?}",
            tables
                .iter()
                .map(|i| i.name.clone())
                .collect::<Vec<String>>()
        );

        Ok(())
    }
}
