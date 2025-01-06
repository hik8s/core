use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryFrom;

#[derive(Debug, Serialize, Deserialize)]
pub struct KubeApiData {
    pub timestamp: i64,
    pub event_type: String,
    pub data: Value,
}

impl TryFrom<Value> for KubeApiData {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl TryFrom<KubeApiData> for Vec<u8> {
    type Error = serde_json::Error;

    fn try_from(data: KubeApiData) -> Result<Self, Self::Error> {
        Ok(serde_json::to_string(&data)?.into_bytes())
    }
}
