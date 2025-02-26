use sqlx::{postgres::PgRow, Executor};

use crate::{log_error, GreptimeConnection};

pub async fn read_records(
    greptime: GreptimeConnection,
    key: &str,
    table_name: &str,
) -> Result<Vec<PgRow>, sqlx::Error> {
    let psql = greptime
        .connect_db(key)
        .await
        .map_err(|e| log_error!(e))
        .unwrap();
    let query = format!("SELECT record_id FROM \"{}\"", table_name);
    let rows = psql.fetch_all(&*query).await?;
    Ok(rows)
}
