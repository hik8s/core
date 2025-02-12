use std::{collections::HashSet, path::Path};

use qdrant_client::qdrant::ScoredPoint;
use shared::connections::{
    dbname::DbName,
    qdrant::connect::{parse_qdrant_value, string_condition, string_filter, QdrantConnection},
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

pub fn write_yaml_files(points: &[ScoredPoint], output_dir: &Path) -> Result<(), std::io::Error> {
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
        let mut resource_version = json_value
            .get("metadata")
            .and_then(|s| s.get("resourceVersion"))
            .and_then(|v| v.as_str());
        resource_version = match resource_version {
            Some(rv) => Some(rv),
            None => json_value
                .get("metadata")
                .and_then(|s| s.get("resource_version"))
                .and_then(|v| v.as_str()),
        };

        let observed_generation = json_value
            .get("status")
            .and_then(|s| s.get("observedGeneration"))
            .and_then(|v| v.as_u64());

        let yaml_string = serde_yaml::to_string(&yaml_value).unwrap();

        let file_name = match (resource_version, observed_generation) {
            (Some(rv), _) => format!("{}-{}-{}-{}.yaml", kind, name, uid, rv),
            (None, Some(gen)) => format!("{}-{}-{}-{}.yaml", kind, name, uid, gen),
            (None, None) => format!("{}-{}-{}-{}.yaml", kind, name, uid, counter),
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
    }

    Ok(())
}
