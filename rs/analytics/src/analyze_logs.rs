use std::collections::HashMap;

use qdrant_client::qdrant::ScoredPoint;
use shared::{connections::dbname::DbName, QdrantConnection};

use crate::{histogram::create_histogram, utils::group_points_by_key};

pub async fn analyze_logs(
    filter_map: HashMap<String, Option<String>>,
    count_key: &str,
    qdrant: &QdrantConnection,
    customer_id: &str,
    limit: u64,
) {
    let db = &DbName::Log;
    let top_k = 10;
    for (filter_key, filter_value) in filter_map {
        let groups =
            group_points_by_key(&filter_key, filter_value, qdrant, db, customer_id, limit).await;
        tracing::debug!("unique groups: {:?}", groups.keys());
        tracing::debug!("unique groups len: {:?}", groups.len());

        let mut groups: Vec<(String, Vec<ScoredPoint>)> = groups.into_iter().collect();
        groups.sort_by(|(_, a_points), (_, b_points)| b_points.len().cmp(&a_points.len()));

        for (filter_value, points) in groups.into_iter().take(top_k) {
            let histogram = create_histogram(count_key, &points, top_k);
            let explanation = format!("\nTop {top_k} most frequent names for {filter_value}:");
            println!("{explanation}\n{histogram}");
        }
    }
}
