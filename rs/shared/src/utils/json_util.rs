use std::io;

use chrono::{DateTime, Utc};
use serde_json::Value;

fn clean_json_string(s: &str) -> String {
    s.trim_matches('"').to_string()
}

pub fn get_as_string(value: &Value, key: &str) -> Result<String, io::Error> {
    let msg = format!("Missing or invalid {key}");
    let error = io::Error::new(io::ErrorKind::InvalidData, msg);
    get_as_option_string(value, key).ok_or(error)
}

pub fn get_as_option_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|u| u.as_str())
        .map(clean_json_string)
}

pub fn get_as_ref<'a>(value: &'a Value, key: &str) -> Result<&'a Value, io::Error> {
    let msg = format!("Missing or invalid {key}");
    let error = io::Error::new(io::ErrorKind::InvalidData, msg);
    value.get(key).ok_or(error)
}

pub fn extract_managed_field_timestamps(metadata: &Value) -> Vec<i64> {
    metadata
        .get("managedFields")
        .and_then(|mf| mf.as_array())
        .map(|managed_fields| {
            managed_fields
                .iter()
                .filter_map(|field| field.get("time").and_then(|t| t.as_str()))
                .filter_map(|time_str| {
                    DateTime::parse_from_rfc3339(time_str)
                        .map(|dt| dt.with_timezone(&Utc).timestamp_micros())
                        .ok()
                })
                .collect::<Vec<i64>>()
        })
        .unwrap_or_default()
}

pub fn extract_timestamp(value: &Value, key: &str) -> i64 {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|time_str| {
            DateTime::parse_from_rfc3339(time_str)
                .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
                .ok()
        })
        .unwrap_or_default()
}

pub fn extract_remove_key(
    json: &mut Value,
    kind: &str,
    metadata_map: &serde_json::Map<String, Value>,
    key: &str,
) -> Option<String> {
    json.as_object_mut().and_then(|m| m.remove(key)).map(|s| {
        let mut map = serde_json::Map::new();
        map.insert("kind".to_string(), Value::String(kind.to_string()));
        map.insert(key.to_string(), s);

        map.insert("metadata".to_string(), Value::Object(metadata_map.clone()));

        serde_yaml::to_string(&Value::Object(map)).unwrap()
    })
}

pub fn create_metadata_map(
    name: &str,
    namespace: &str,
    uid: &str,
) -> serde_json::Map<String, Value> {
    let mut metadata_map = serde_json::Map::new();
    metadata_map.insert("name".to_string(), Value::String(name.to_string()));
    metadata_map.insert(
        "namespace".to_string(),
        Value::String(namespace.to_string()),
    );
    metadata_map.insert("uid".to_string(), Value::String(uid.to_string()));
    metadata_map
}
