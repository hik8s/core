use fluvio::dataplane::record::ConsumerRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::PartialEq;
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KubeEventType {
    Apply,
    InitApply,
    Delete,
}

impl fmt::Display for KubeEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KubeEventType::Apply => write!(f, "apply"),
            KubeEventType::InitApply => write!(f, "initapply"),
            KubeEventType::Delete => write!(f, "delete"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KubeApiData {
    pub timestamp: i64,
    pub event_type: KubeEventType,
    pub json: Value,
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

impl TryFrom<ConsumerRecord> for KubeApiData {
    type Error = serde_json::Error;

    fn try_from(record: ConsumerRecord) -> Result<Self, Self::Error> {
        let payload = record.value();
        let data_str = String::from_utf8_lossy(payload);
        serde_json::from_str::<KubeApiData>(&data_str)
    }
}
