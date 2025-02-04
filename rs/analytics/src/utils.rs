use std::{collections::HashSet, path::Path};

use qdrant_client::qdrant::ScoredPoint;
use shared::connections::{
    dbname::DbName,
    qdrant::connect::{string_condition, string_filter, QdrantConnection},
};

pub fn unique_values(key: &str, points: &[ScoredPoint]) -> HashSet<String> {
    points
        .iter()
        .filter_map(|point| {
            point
                .payload
                .get(key)
                .map(|v| v.as_str().unwrap().to_string())
        })
        .collect()
}

fn parse_data(data: &qdrant_client::qdrant::Value) -> (serde_yaml::Value, serde_json::Value) {
    let content = data.as_str().unwrap();

    // Parse YAML first
    let yaml: serde_yaml::Value = serde_yaml::from_str(content).unwrap();

    // Convert to JSON without re-parsing
    let json = serde_json::to_value(&yaml).unwrap();

    (yaml, json)
}

pub fn write_yaml_files(points: &[ScoredPoint], output_dir: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(output_dir)?;

    for (counter, point) in points.iter().enumerate() {
        let data = point.payload.get("data").unwrap();
        let (yaml_value, json_value) = parse_data(data);

        let kind = json_value
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let name = json_value
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let observed_generation = json_value
            .get("status")
            .and_then(|s| s.get("observedGeneration"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let yaml_string = serde_yaml::to_string(&yaml_value).unwrap();
        let file_name = format!("{}-{}-{}-{}.yaml", kind, name, observed_generation, counter);
        let file_path = output_dir.join(file_name);

        std::fs::write(file_path, yaml_string)?;
    }
    Ok(())
}

pub async fn write_deployment_status_yaml(
    dir: &str,
    name: &str,
    qdrant: &QdrantConnection,
    customer_id: &str,
    limit: u64,
) -> Result<(), std::io::Error> {
    let db = DbName::Resource;
    let kind = "Deployment";
    let data_type = "status";
    let subdir = format!(
        "{}_{}_{}",
        kind.to_lowercase(),
        name.to_lowercase(),
        data_type.to_lowercase()
    );
    let output_dir = Path::new(dir).join(subdir);
    std::fs::create_dir_all(&output_dir).unwrap();

    let mut filter = string_filter("kind", kind);
    filter.must.push(string_condition("name", name));
    filter.must.push(string_condition("data_type", data_type));

    let points = qdrant
        .query_points(&db, customer_id, Some(filter), limit, true)
        .await
        .unwrap();

    write_yaml_files(&points, &output_dir).unwrap();

    Ok(())
}
