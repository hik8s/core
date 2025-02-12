use shared::connections::{dbname::DbName, qdrant::connect::QdrantConnection};

use crate::{histogram::create_histograms, utils::write_resource_yaml};

pub async fn analyze_resource(qdrant: &QdrantConnection, customer_id: &str, limit: u64) {
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

    // write deployment
    let write = false;
    if write {
        write_resource_yaml(
            ".prod1",
            "test1",
            "Deployment",
            &["status", "metadata", "spec"],
            qdrant,
            customer_id,
            limit,
        )
        .await
        .unwrap();
        write_resource_yaml(
            ".prod1",
            "test1-656b95f57-zjln7",
            "Pod",
            &["status", "metadata", "spec"],
            qdrant,
            customer_id,
            limit,
        )
        .await
        .unwrap();
    }

    // histograms
    create_histograms(
        "kind",
        "name",
        kinds,
        qdrant,
        &DbName::Resource,
        customer_id,
        limit,
        10,
    )
    .await
    .unwrap();
}
