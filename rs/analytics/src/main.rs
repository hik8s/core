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
    connections::{qdrant::connect::QdrantConnection, redis::connect::RedisConnection},
    get_env_var,
    tracing::setup::setup_tracing,
};
use utils::create_map;

#[tokio::main]
async fn main() {
    setup_tracing(false);
    let limit = 1000000;
    let run_analyze_resource = false;
    let run_analyze_log = true;
    let run_analyze_state = true;

    env::set_var("QDRANT_HOST", "dev.qdrant.hik8s.ai");
    let customer_id = get_env_var("ANALYTICS_CLIENT_ID").unwrap();
    let qdrant = QdrantConnection::new().await.unwrap();

    let filter_map = create_map("namespace", Some("examples"));
    if run_analyze_resource {
        analyze_resource(filter_map, "name", &qdrant, &customer_id, limit).await;
    }

    let filter_map = create_map("namespace", Some("examples"));
    if run_analyze_log {
        analyze_logs(filter_map, "key", &qdrant, &customer_id, limit).await;
    }

    // env::set_var("REDIS_HOST", "dev.qdrant.hik8s.ai");
    let mut redis = RedisConnection::new().unwrap();
    if run_analyze_state {
        analyze_state(&mut redis, &customer_id).await;
    }
}
