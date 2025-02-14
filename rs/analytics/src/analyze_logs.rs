use qdrant_client::qdrant::ScoredPoint;
use shared::connections::{dbname::DbName, qdrant::connect::QdrantConnection};

use crate::{histogram::create_histogram, utils::group_points_by_key};

pub async fn analyze_logs(
    namespace: Option<&str>,
    qdrant: &QdrantConnection,
    customer_id: &str,
    limit: u64,
) {
    let filter_key = "namespace";
    let count_key = "key";
    let db = &DbName::Log;
    let top_k = 10;

    let groups = group_points_by_key(filter_key, namespace, qdrant, db, customer_id, limit).await;
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
