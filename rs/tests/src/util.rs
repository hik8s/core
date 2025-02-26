use std::{collections::HashMap, path::Path};

use data_intake::error::DataIntakeError;
use rocket::{http::Status, local::asynchronous::Client};
use shared::{
    types::kubeapidata::KubeEventType, utils::mock::mock_client::post_test_batch, DbName,
};
use uuid7::uuid4;

fn parse_apply_filename(filename: &str) -> String {
    if filename.starts_with("apply_") {
        return KubeEventType::Apply.to_string();
    }
    if filename.starts_with("delete_") {
        return KubeEventType::Delete.to_string();
    }
    if filename.starts_with("initapply_") {
        return KubeEventType::InitApply.to_string();
    }
    "apply".to_string()
}

pub fn read_yaml_files(path: &Path) -> Result<Vec<serde_json::Value>, std::io::Error> {
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

pub fn replace_resource_uids(
    resources: &mut [serde_json::Value],
    dbname: &DbName,
) -> HashMap<String, String> {
    let mut uid_map: HashMap<String, String> = HashMap::new();

    let key = match dbname {
        DbName::Resource => "ownerReferences",
        DbName::CustomResource => "ownerReferences",
        DbName::Event => "involvedObject",
        _ => "",
    };

    for resource in resources.iter_mut() {
        if let Some(json) = resource.get_mut("json") {
            if let Some(metadata) = json.as_object_mut().and_then(|obj| obj.get_mut("metadata")) {
                if let Some(metadata_obj) = metadata.as_object_mut() {
                    let current_uid = metadata_obj
                        .get("uid")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let new_uid = uid_map
                        .entry(current_uid)
                        .or_insert_with(|| uuid4().to_string())
                        .clone();
                    metadata_obj.insert(
                        "uid".to_string(),
                        serde_json::Value::String(new_uid.clone()),
                    );

                    if let Some(owner_refs) = metadata_obj.get_mut(key) {
                        if let Some(owner_refs_array) = owner_refs.as_array_mut() {
                            for owner_ref in owner_refs_array {
                                if let Some(owner_ref_obj) = owner_ref.as_object_mut() {
                                    let original_owner_uid = owner_ref_obj
                                        .get("uid")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    let owner_uid = uid_map
                                        .entry(original_owner_uid)
                                        .or_insert_with(|| uuid4().to_string())
                                        .clone();

                                    owner_ref_obj.insert(
                                        "uid".to_string(),
                                        serde_json::Value::String(owner_uid),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    uid_map
}
