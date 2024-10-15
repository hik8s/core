use sqlx::{postgres::PgRow, Executor};

use crate::{
    connections::{db_name::get_db_name, greptime::connect::GreptimeConnection},
    get_env_var,
};

pub async fn read_records(
    greptime: GreptimeConnection,
    table_name: &str,
) -> Result<Vec<PgRow>, sqlx::Error> {
    let customer_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
    let db_name = get_db_name(&customer_id);
    let psql = greptime.connect_db(&db_name).await.unwrap();
    let query = format!("SELECT record_id FROM \"{}\"", table_name);
    let rows = psql.fetch_all(&*query).await?;
    Ok(rows)
}
