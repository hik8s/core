use sqlx::{postgres::PgRow, Executor};

use crate::{connections::greptime::connect::GreptimeConnection, log_error};

pub async fn read_records(
    greptime: GreptimeConnection,
    db_name: &str,
    table_name: &str,
) -> Result<Vec<PgRow>, sqlx::Error> {
    let psql = greptime
        .connect_db(&db_name)
        .await
        .map_err(|e| log_error!(e))
        .unwrap();
    let query = format!("SELECT record_id FROM \"{}\"", table_name);
    let rows = psql.fetch_all(&*query).await?;
    Ok(rows)
}
