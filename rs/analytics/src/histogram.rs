use std::collections::HashMap;

use qdrant_client::qdrant::ScoredPoint;
use shared::connections::{
    dbname::DbName,
    qdrant::connect::{string_filter, QdrantConnection},
};
use tabled::{Table, Tabled};
#[derive(Tabled)]
struct HistogramRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Count")]
    count: usize,
    #[tabled(rename = "Percentage")]
    percentage: String,
}

pub fn create_histogram(points: &[ScoredPoint], kind: &str, top_k: usize) -> String {
    // Create frequency map
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for point in points {
        if let Some(name) = point.payload.get("name").and_then(|n| n.as_str()) {
            *name_counts.entry(name.to_string()).or_default() += 1;
        }
    }

    // Convert to vec and sort
    let mut counts: Vec<(String, usize)> = name_counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1));

    // Create histogram rows
    let mut histogram_rows = Vec::new();
    for (name, count) in counts.iter().take(top_k) {
        let percentage = format!("{:.1}%", (*count as f64 / points.len() as f64) * 100.0);
        histogram_rows.push(HistogramRow {
            name: name.clone(),
            count: *count,
            percentage,
        });
    }
    histogram_rows.push(HistogramRow {
        name: kind.to_string(),
        count: points.len(),
        percentage: "100%".to_string(),
    });

    format!(
        "\nTop 10 most frequent names for {}:\n{}",
        kind,
        Table::new(histogram_rows)
    )
}

pub async fn resource_histograms(
    kinds: Vec<&str>,
    qdrant: &QdrantConnection,
    db: &DbName,
    customer_id: &str,
    limit: u64,
    top_k: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for kind in kinds {
        let filter = string_filter("kind", kind);
        let points = qdrant
            .query_points(db, customer_id, Some(filter), limit, true)
            .await?;

        println!("{}", create_histogram(&points, kind, top_k));
    }
    Ok(())
}
