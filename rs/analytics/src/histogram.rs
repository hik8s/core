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
const MAX_NAME_LENGTH: usize = 80;

pub fn create_histogram(key: &str, points: &[ScoredPoint], top_k: usize) -> Table {
    // Create frequency map
    let mut value_counts: HashMap<String, usize> = HashMap::new();
    for point in points {
        if let Some(value) = point.payload.get(key).and_then(|n| n.as_str()) {
            *value_counts.entry(value.to_string()).or_default() += 1;
        }
    }

    // Convert to vec and sort
    let mut counts: Vec<(String, usize)> = value_counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1));

    // Create histogram rows
    let mut histogram_rows = Vec::new();
    for (name, count) in counts.iter().take(top_k) {
        let percentage = format!("{:.1}%", (*count as f64 / points.len() as f64) * 100.0);
        let truncated_name = if name.len() > MAX_NAME_LENGTH {
            format!("{}...", &name[..MAX_NAME_LENGTH])
        } else {
            name.to_owned()
        };
        histogram_rows.push(HistogramRow {
            name: truncated_name,
            count: *count,
            percentage,
        });
    }
    histogram_rows.push(HistogramRow {
        name: "Total".to_string(),
        count: points.len(),
        percentage: "100%".to_string(),
    });
    Table::new(histogram_rows)
}

pub async fn create_histograms(
    key_separate: &str,
    key_aggregate: &str,
    values: Vec<&str>,
    qdrant: &QdrantConnection,
    db: &DbName,
    customer_id: &str,
    limit: u64,
    top_k: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for value in values {
        let filter = string_filter(key_separate, value);
        let points = qdrant
            .query_points(db, customer_id, Some(filter), limit, true)
            .await?;
        let histogram = create_histogram(key_aggregate, &points, top_k);
        let histogram_string =
            format!("\nTop {top_k} most frequent names for {value}:\n{histogram}",);
        println!("{histogram_string}");
    }
    Ok(())
}
