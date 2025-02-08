pub mod histogram;
pub mod utils;

use std::env;

use histogram::resource_histograms;
use shared::{
    connections::{dbname::DbName, qdrant::connect::QdrantConnection},
    get_env_var,
    tracing::setup::setup_tracing,
};

use utils::write_resource_status_yaml;

#[tokio::main]
async fn main() {
    setup_tracing(false);
    env::set_var("QDRANT_HOST", "prod.qdrant.hik8s.ai");
    let kinds = vec![
        "Ingress",
        "Pod",
        "Namespace",
        "ServiceAccount",
        "StorageClass",
        "DaemonSet",
        "ClusterRole",
        "Service",
        "Node",
        "ClusterRoleBinding",
        "Role",
        "StatefulSet",
        "Deployment",
    ];
    let customer_id = get_env_var("ANALYTICS_CLIENT_ID").unwrap();
    let qdrant = QdrantConnection::new().await.unwrap();

    // write deployment
    let write = false;
    if write {
        write_resource_status_yaml(
            ".giant-yaml",
            "policy-meta-operator",
            "Deployment",
            &["status"],
            &qdrant,
            &customer_id,
            10000,
        )
        .await
        .unwrap();
        write_resource_status_yaml(
            ".giant-yaml",
            "policy-meta-operator-7496ffdfdb-hfzqk",
            "Pod",
            &["status", "metadata", "spec"],
            &qdrant,
            &customer_id,
            10000,
        )
        .await
        .unwrap();
    }

    // histograms
    resource_histograms(kinds, &qdrant, &DbName::Resource, &customer_id, 150000, 10)
        .await
        .unwrap();
}
