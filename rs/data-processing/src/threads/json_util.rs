use std::io;

use chrono::{DateTime, Utc};
use serde_json::Value;

pub fn clean_json_string(s: &str) -> String {
    s.trim_matches('"').to_string()
}

pub fn get_as_string(value: &Value, key: &str) -> Result<String, io::Error> {
    let msg = format!("Missing or invalid {key}");
    let error = io::Error::new(io::ErrorKind::InvalidData, msg);
    get_as_option_string(value, key).ok_or_else(|| error)
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
    value.get(key).ok_or_else(|| error)
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

pub fn extract_creation_timestamp(metadata: &Value) -> i64 {
    metadata
        .get("creationTimestamp")
        .and_then(|ct| ct.as_str())
        .and_then(|ct_str| {
            DateTime::parse_from_rfc3339(ct_str)
                .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
                .ok()
        })
        .unwrap_or_default()
}
