use sqlx::{postgres::PgRow, Executor};

use crate::{log_error, DbName, GreptimeConnection};

pub async fn read_records(
    greptime: GreptimeConnection,
    db: &DbName,
    customer_id: &str,
    table_name: &str,
) -> Result<Vec<PgRow>, sqlx::Error> {
    let psql = greptime
        .connect_db(db, customer_id)
        .await
        .map_err(|e| log_error!(e))
        .unwrap();
    let query = format!("SELECT record_id FROM \"{}\"", table_name);
    let rows = psql.fetch_all(&*query).await?;
    Ok(rows)
}
