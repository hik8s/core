use std::{collections::HashMap, path::Path};

use qdrant_client::qdrant::ScoredPoint;
use shared::connections::{
    dbname::DbName,
    qdrant::connect::{parse_qdrant_value, string_condition, string_filter, QdrantConnection},
};

pub async fn group_points_by_key(
    key: &str,
    value: Option<&str>,
    qdrant: &QdrantConnection,
    db: &DbName,
    customer_id: &str,
    limit: u64,
) -> HashMap<String, Vec<ScoredPoint>> {
    let filter = value.map(|v| string_filter(key, v));
    let points = qdrant
        .query_points(db, customer_id, filter, limit, true)
        .await
        .unwrap();

    let mut grouped_points: HashMap<String, Vec<ScoredPoint>> = HashMap::new();

    for point in points {
        if let Some(value) = point.payload.get(key).and_then(|v| v.as_str()) {
            grouped_points
                .entry(value.to_string())
                .or_default()
                .push(point.clone());
        }
    }
    grouped_points
}

pub fn write_yaml_files(
    points: &[ScoredPoint],
    output_dir: &Path,
    data_type: &str,
) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(output_dir)?;

    for (counter, point) in points.iter().enumerate() {
        let data = point.payload.get("data").unwrap();
        let (yaml_value, json_value) = parse_qdrant_value(data);

        let kind = json_value
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let name = json_value
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // TODO: replace uid and observed_generation with resourceVersion
        let uid = json_value
            .get("metadata")
            .and_then(|m| m.get("uid"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let resource_version = json_value
            .get("metadata")
            .and_then(|s| s.get("resourceVersion"))
            .and_then(|v| v.as_str());

        let observed_generation = json_value
            .get("status")
            .and_then(|s| s.get("observedGeneration"))
            .and_then(|v| v.as_u64());

        let yaml_string = serde_yaml::to_string(&yaml_value).unwrap();
        let kind = kind.to_lowercase();
        let file_name = match (resource_version, observed_generation) {
            (Some(rv), _) => format!("{}_{}_{}_{}_{}.yaml", kind, name, uid, rv, data_type),
            (None, Some(gen)) => format!("{}_{}_{}_{}_{}.yaml", kind, name, uid, gen, data_type),
            (None, None) => format!("{}_{}_{}_{}_{}.yaml", kind, name, uid, counter, data_type),
        };

        let file_path = output_dir.join(file_name);

        std::fs::write(file_path, yaml_string)?;
    }
    Ok(())
}

pub async fn write_resource_yaml(
    dir: &str,
    name: &str,
    kind: &str,
    data_types: &[&str],
    qdrant: &QdrantConnection,
    customer_id: &str,
    limit: u64,
) -> Result<(), std::io::Error> {
    let db = DbName::Resource;
    for data_type in data_types {
        let subdir = format!("{}_{}", kind.to_lowercase(), name.to_lowercase(),);
        let output_dir = Path::new(dir).join(subdir);
        std::fs::create_dir_all(&output_dir).unwrap();

        let mut filter = string_filter("kind", kind);
        filter.must.push(string_condition("name", name));
        filter.must.push(string_condition("data_type", data_type));

        let points = qdrant
            .query_points(&db, customer_id, Some(filter), limit, true)
            .await
            .unwrap();

        write_yaml_files(&points, &output_dir, data_type).unwrap();
    }

    Ok(())
}
