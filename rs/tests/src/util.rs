use std::path::{Path, PathBuf};

use data_intake::error::DataIntakeError;
use rocket::{http::Status, local::asynchronous::Client};
use shared::utils::mock::mock_client::post_test_batch;

fn parse_apply_filename(filename: &str) -> String {
    if filename.starts_with("apply_") {
        return "apply".to_string();
    }
    if filename.starts_with("delete_") {
        return "delete".to_string();
    }
    if filename.starts_with("initapply_") {
        return "initapply".to_string();
    }
    "apply".to_string()
}

pub fn read_yaml_files(path: &PathBuf) -> Result<Vec<serde_json::Value>, std::io::Error> {
    let pattern = format!("{}/*.yaml", path.to_str().unwrap());
    let mut json_values = Vec::new();

    for entry in glob::glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let event_type = parse_apply_filename(path.file_name().unwrap().to_str().unwrap());
                let yaml_content = std::fs::read_to_string(&path)?;
                let json_value: serde_json::Value = serde_yaml::from_str(&yaml_content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                json_values.push(wrap_kubeapi_data(json_value, event_type.as_str()));
            }
            Err(e) => tracing::error!("Error reading file: {:?}", e),
        }
    }

    Ok(json_values)
}

fn wrap_kubeapi_data(json: serde_json::Value, event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "timestamp": chrono::Utc::now().timestamp(),
        "event_type": event_type,
        "json": json,
    })
}

pub async fn process_resource_files(client: Client, route: &str) -> Result<(), DataIntakeError> {
    // Define directories to process
    const BATCH_SIZE: usize = 100; // Adjust as needed

    let testdata_dir = Path::new(".testdata");
    let path = testdata_dir.join(route);

    // Collect all JSON values for this route
    let files = read_yaml_files(&path).unwrap();

    if !files.is_empty() {
        let route = route.to_string();

        // Process in batches
        for batch in files.chunks(BATCH_SIZE) {
            tracing::info!(
                "Processing route: {}: batch of {} items",
                route,
                batch.len()
            );
            let status = post_test_batch(&client, &format!("/{route}"), batch.to_vec()).await;
            assert_eq!(status, Status::Ok);
        }
    }
    Ok(())
}
