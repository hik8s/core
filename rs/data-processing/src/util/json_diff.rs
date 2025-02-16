use serde_json::Value;

pub const RESOURCE_STATUS_ALLOWED_KEYS: &[&str] = &[
    "status.conditions",
    "status.containerStatuses",
    "status.leader.spu",
];

pub const RESOURCE_IGNORE_KEYS: &[&str] = &["metadata.resourceVersion", "metadata.managedFields"];

#[derive(Debug, PartialEq)]
pub struct Difference {
    pub path: String,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
    pub diff_type: DiffType,
}

#[derive(Debug, PartialEq)]
pub enum DiffType {
    Added,
    Removed,
    Modified,
}

pub fn compare_json(old: &Value, new: &Value) -> Vec<Difference> {
    let mut differences = Vec::new();
    compare_values(old, new, String::new(), &mut differences);
    differences
}

fn compare_values(old: &Value, new: &Value, path: String, differences: &mut Vec<Difference>) {
    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            compare_objects(old_map, new_map, path, differences);
        }
        (Value::Array(old_array), Value::Array(new_array)) => {
            compare_arrays(old_array, new_array, path, differences);
        }
        (old_val, new_val) if old_val != new_val => {
            // Only check whitelist for modified values
            if is_whitelisted_status_key(&path, old_val) {
                differences.push(Difference {
                    path,
                    old_value: Some(old_val.clone()),
                    new_value: Some(new_val.clone()),
                    diff_type: DiffType::Modified,
                });
            }
        }
        _ => {}
    }
}

fn is_whitelisted_status_key(path: &str, value: &Value) -> bool {
    if RESOURCE_IGNORE_KEYS
        .iter()
        .any(|&ignore_key| path.starts_with(ignore_key))
    {
        return false;
    }

    if !path.starts_with("status") {
        return true; // Not a status field, allow all
    }
    let is_whitelisted = RESOURCE_STATUS_ALLOWED_KEYS
        .iter()
        .any(|&allowed_key| path.starts_with(allowed_key));

    // Exclude only if both conditions are true:
    // 1. Not in whitelist
    // 2. Is a number
    if !is_whitelisted && value.is_number() {
        tracing::warn!(
            "Ignoring non-whitelisted numeric status key: {} with value: {:?}",
            path,
            value
        );
        return false;
    }

    true
}

fn compare_objects(
    old_map: &serde_json::Map<String, Value>,
    new_map: &serde_json::Map<String, Value>,
    path: String,
    differences: &mut Vec<Difference>,
) {
    // Find removed and modified
    for (key, old_value) in old_map {
        let new_value = new_map.get(key);
        let new_path = if path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", path, key)
        };
        match new_value {
            Some(new_val) => {
                compare_values(old_value, new_val, new_path, differences);
            }
            None => {
                differences.push(Difference {
                    path: new_path,
                    old_value: Some(old_value.clone()),
                    new_value: None,
                    diff_type: DiffType::Removed,
                });
            }
        }
    }

    // Find added
    for (key, new_value) in new_map {
        if !old_map.contains_key(key) {
            let new_path = if path.is_empty() {
                key.to_string()
            } else {
                format!("{}.{}", path, key)
            };

            differences.push(Difference {
                path: new_path,
                old_value: None,
                new_value: Some(new_value.clone()),
                diff_type: DiffType::Added,
            });
        }
    }
}

fn compare_arrays(
    old_array: &[Value],
    new_array: &[Value],
    path: String,
    differences: &mut Vec<Difference>,
) {
    for (index, (old_value, new_value)) in old_array.iter().zip(new_array.iter()).enumerate() {
        let new_path = format!("{}[{}]", path, index);
        compare_values(old_value, new_value, new_path, differences);
    }

    // Handle different lengths
    match old_array.len().cmp(&new_array.len()) {
        std::cmp::Ordering::Less => {
            // Handle added elements
            for (i, new_value) in new_array.iter().enumerate().skip(old_array.len()) {
                differences.push(Difference {
                    path: format!("{}[{}]", path, i),
                    old_value: None,
                    new_value: Some(new_value.clone()),
                    diff_type: DiffType::Added,
                });
            }
        }
        std::cmp::Ordering::Greater => {
            // Handle removed elements
            for (i, old_value) in old_array.iter().enumerate().skip(new_array.len()) {
                differences.push(Difference {
                    path: format!("{}[{}]", path, i),
                    old_value: Some(old_value.clone()),
                    new_value: None,
                    diff_type: DiffType::Removed,
                });
            }
        }
        std::cmp::Ordering::Equal => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use shared::setup_tracing;

    #[test]
    fn test_compare_partition() {
        setup_tracing(false);
        let old = json!({
            "apiVersion": "fluvio.infinyon.com/v1",
            "kind": "Partition",
            "metadata": {
                "name": "classes-0",
                "namespace": "fluvio-stag",
                "uid": "ea9dc3ff-0900-4f4d-b56b-b34011d2ba1f",
                "resourceVersion": "66423481",
                "managedFields": {
                    "some_test_array_object": [{"number": 123}],
                    "some_number": 123,
                    "subresource": "status",
                    "time": "2024-12-15T19:52:08Z"
                },
            },
            "spec": {
                "compressionType": "Any",
                "deduplication": null,
                "leader": 100,
                "replicas": [100],
                "system": false
            },
            "status": {
                "baseOffset": 0,
                "isBeingDeleted": false,
                "leader": {
                    "hw": 16309,
                    "leo": 16309,
                    "spu": 100
                },
                "lsr": 0,
                "replicas": [],
                "resolution": "Online",
                "size": 8137666
            }
        });

        let new = json!({
            "apiVersion": "fluvio.infinyon.com/v1",
            "kind": "Partition",
            "metadata": {
                "name": "classes-0",
                "namespace": "fluvio-stag",
                "uid": "ea9dc3ff-0900-4f4d-b56b-b34011d2ba1f",
                "resourceVersion": "66423482", // should be considered
                "managedFields": {
                    "some_test_array_object": [{"number": 567}],
                    "some_number": 567,
                    "subresource": "status",
                    "time": "2024-12-15T19:63:08Z"
                },
            },
            "spec": {
                "compressionType": "Any",
                "deduplication": null,
                "leader": 101, // should be considered
                "replicas": [100],
                "system": false
            },
            "status": {
                "baseOffset": 0,
                "isBeingDeleted": false,
                "leader": {
                    "hw": 16319, // should be ignored
                    "leo": 16319, // should be ignored
                    "spu": 101 // should be considered
                },
                "lsr": 0,
                "replicas": [],
                "resolution": "Offline", // should be considered
                "size": 8137999 // should be ignored
            }
        });

        let differences = compare_json(&old, &new);

        let expected = vec![
            Difference {
                path: "spec.leader".to_string(),
                old_value: Some(json!(100)),
                new_value: Some(json!(101)),
                diff_type: DiffType::Modified,
            },
            Difference {
                path: "status.leader.spu".to_string(),
                old_value: Some(json!(100)),
                new_value: Some(json!(101)),
                diff_type: DiffType::Modified,
            },
            Difference {
                path: "status.resolution".to_string(),
                old_value: Some(json!("Online")),
                new_value: Some(json!("Offline")),
                diff_type: DiffType::Modified,
            },
        ];
        assert_eq!(differences.len(), expected.len());
        assert_eq!(differences, expected);
    }

    #[test]
    fn test_compare_pod_status() {
        setup_tracing(false);
        let old = json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "thisisatest-5497c554fc-dqlnd",
                "namespace": "examples",
                "resourceVersion": "141509498"
            },
            "status": {
                "conditions": [
                    {
                        "lastTransitionTime": "2024-12-11T17:00:21Z",
                        "status": "True",
                        "type": "PodScheduled"
                    }
                ],
                "containerStatuses": [
                    {
                        "name": "hello-server",
                        "ready": true,
                        "restartCount": 0,
                        "state": {
                            "waiting": {
                                "reason": "ContainerCreating"
                            }
                        }
                    }
                ],
                "hostIP": "123.456.123.321",
                "hostIPs": [{"ip": "123.456.123.321"}],
                "some_test_array_object": [{"number": 123}],
                "some_test_array": [123],
                "phase": "Pending",
                "qosClass": "Burstable"
            }
        });

        let new = json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "thisisatest-5497c554fc-dqlnd",
                "namespace": "examples",
                "resourceVersion": "141509504"
            },
            "status": {
                "conditions": [
                    {
                        "lastTransitionTime": "2024-12-11T17:00:21Z",
                        "status": "False",
                        "type": "PodReadyToStartContainers"
                    },
                    {
                        "lastTransitionTime": "2024-12-11T17:00:21Z",
                        "status": "True",
                        "type": "Initialized"
                    }
                ],
                "containerStatuses": [
                    {
                        "name": "hello-server",
                        "ready": false,
                        "restartCount": 1,
                        "state": {
                            "waiting": {
                                "reason": "ContainerCreating"
                            }
                        }
                    }
                ],
                "hostIP": "123.456.123.654",
                "hostIPs": [{"ip": "123.456.123.654"}],
                "some_test_array_object": [{"number": 567}],
                "some_test_array": [567],
                "phase": "Pending",
                "qosClass": "Burstable",
            }
        });

        let differences = compare_json(&old, &new);

        let expected = vec![
            Difference {
                path: "status.conditions[0].status".to_string(),
                old_value: Some(json!("True")),
                new_value: Some(json!("False")),
                diff_type: DiffType::Modified,
            },
            Difference {
                path: "status.conditions[0].type".to_string(),
                old_value: Some(json!("PodScheduled")),
                new_value: Some(json!("PodReadyToStartContainers")),
                diff_type: DiffType::Modified,
            },
            Difference {
                path: "status.conditions[1]".to_string(),
                old_value: None,
                new_value: Some(json!({
                    "lastTransitionTime": "2024-12-11T17:00:21Z",
                    "status": "True",
                    "type": "Initialized"
                })),
                diff_type: DiffType::Added,
            },
            Difference {
                path: "status.containerStatuses[0].ready".to_string(),
                old_value: Some(json!(true)),
                new_value: Some(json!(false)),
                diff_type: DiffType::Modified,
            },
            Difference {
                path: "status.containerStatuses[0].restartCount".to_string(),
                old_value: Some(json!(0)),
                new_value: Some(json!(1)),
                diff_type: DiffType::Modified,
            },
            Difference {
                path: "status.hostIP".to_string(),
                old_value: Some(json!("123.456.123.321")),
                new_value: Some(json!("123.456.123.654")),
                diff_type: DiffType::Modified,
            },
            Difference {
                path: "status.hostIPs[0].ip".to_string(),
                old_value: Some(json!("123.456.123.321")),
                new_value: Some(json!("123.456.123.654")),
                diff_type: DiffType::Modified,
            },
        ];
        assert_eq!(differences.len(), expected.len());
        assert_eq!(differences, expected);
    }
}
