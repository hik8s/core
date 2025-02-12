use shared::connections::{dbname::DbName, qdrant::connect::QdrantConnection};

use crate::{
    histogram::create_histogram,
    utils::{group_points_by_key, write_resource_yaml},
};

pub async fn analyze_resource(qdrant: &QdrantConnection, customer_id: &str, limit: u64) {
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

    let filter_key = "kind";
    let count_key = "name";
    let db = &DbName::Resource;
    let top_k = 10;

    // histograms
    let groups = group_points_by_key(filter_key, qdrant, db, customer_id, limit).await;
    tracing::debug!("unique groups: {:?}", groups.keys());
    tracing::debug!("unique groups len: {:?}", groups.len());

    for (group, points) in groups {
        let histogram = create_histogram(count_key, &points, top_k);
        let explanation = format!("\nTop {top_k} most frequent names for {group}:");
        println!("{explanation}\n{histogram}");
    }
}
