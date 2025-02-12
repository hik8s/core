pub mod analyze_logs;
pub mod analyze_resource;
pub mod histogram;
pub mod utils;

use std::env;

use analyze_resource::analyze_resource;
use shared::{
    connections::qdrant::connect::QdrantConnection, get_env_var, tracing::setup::setup_tracing,
};

#[tokio::main]
async fn main() {
    setup_tracing(false);

    let run_analyze_resource = false;

    env::set_var("QDRANT_HOST", "dev.qdrant.hik8s.ai");
    let customer_id = get_env_var("ANALYTICS_CLIENT_ID").unwrap();
    let qdrant = QdrantConnection::new().await.unwrap();
    if run_analyze_resource {
        analyze_resource(&qdrant, &customer_id).await;
    }
}
