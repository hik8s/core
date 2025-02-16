use std::collections::HashMap;

use shared::{DbName, QdrantConnection};

use crate::{
    histogram::create_histogram,
    utils::{group_points_by_key, write_resource_yaml},
};

pub async fn analyze_resource(
    filter_map: HashMap<String, Option<String>>,
    count_key: &str,
    qdrant: &QdrantConnection,
    customer_id: &str,
    limit: u64,
) {
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

    let db = &DbName::Resource;
    let top_k = 10;

    // histograms
    for (filter_key, filter_value) in filter_map {
        let groups =
            group_points_by_key(&filter_key, filter_value, qdrant, db, customer_id, limit).await;
        tracing::debug!("unique groups: {:?}", groups.keys());
        tracing::debug!("unique groups len: {:?}", groups.len());

        for (group, points) in groups {
            let histogram = create_histogram(count_key, &points, top_k);
            let explanation = format!("\nTop {top_k} most frequent names for {group}:");
            println!("{explanation}\n{histogram}");
        }
    }
}
