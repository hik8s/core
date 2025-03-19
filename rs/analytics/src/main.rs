pub mod analyze_logs;
pub mod analyze_resource;
pub mod analyze_state;
pub mod histogram;
pub mod utils;

use std::env;

use analyze_logs::analyze_logs;
use analyze_resource::analyze_resource;
use analyze_state::analyze_state;
use shared::{
    get_env_var, setup_tracing, DbName, GreptimeConnection, QdrantConnection, RedisConnection,
};
use utils::create_map;

#[tokio::main]
async fn main() {
    setup_tracing(false);
    let limit = 1000000;
    let run_analyze_resource = false;
    let run_analyze_log = false;
    let run_analyze_state = false;
    let run_export_greptime = false;

    env::set_var("QDRANT_HOST", "dev.qdrant.hik8s.ai");
    env::set_var("GREPTIMEDB_HOST", "dev.greptime.hik8s.ai");

    let customer_id = get_env_var("ANALYTICS_CLIENT_ID").unwrap();
    let db_resource = DbName::Resource.id(&customer_id);
    let db_log = DbName::Log.id(&customer_id);
    let qdrant = QdrantConnection::new().await.unwrap();

    // group by namespace and filter "examples"
    // let group_by = create_map("namespace", Some("examples"));

    // group by kind and filter "Namespace"
    let group_by = create_map("kind", Some("Namespace"));
    if run_analyze_resource {
        analyze_resource(group_by, "name", &qdrant, &db_resource, limit).await;
    }

    let group_by = create_map("namespace", Some("examples"));
    if run_analyze_log {
        analyze_logs(group_by, "key", &qdrant, &db_log, limit).await;
    }

    // env::set_var("REDIS_HOST", "dev.qdrant.hik8s.ai");
    if run_analyze_state {
        let mut redis = RedisConnection::new().unwrap();
        analyze_state(&mut redis, &db_log).await;
    }

    if run_export_greptime {
        let greptime = GreptimeConnection::new().await.unwrap();
        let tables = greptime
            .list_tables(&db_resource, None, None, false)
            .await
            .unwrap();

        let table = tables.first().unwrap();
        let rows = greptime
            .query(&db_resource, &table.format_name(), "*")
            .await
            .unwrap();

        tracing::debug!("Tables {tables:#?}");
        tracing::info!("Rows {rows:#?}");
    }
}
