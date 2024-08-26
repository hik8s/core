use chrono::{NaiveDateTime, ParseError, TimeZone, Utc};
use fluvio::dataplane::record::ConsumerRecord;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::convert::TryFrom;
use thiserror::Error;
use tracing::warn;
use uuid7::uuid7;

const DEFAULT_TS: &str = "1970-01-01T00:00:00Z";

#[derive(Debug, Error)]
pub enum LogParseError {
    #[error("Timestamp not found in record: {0}")]
    MissingTimestamp(String),
    #[error("Failed to parse timestamp for record {0}: {1}")]
    ParseError(String, ParseError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogRecord {
    pub timestamp: i64,
    pub message: String,
    pub record_id: String,
    pub key: String,
}

impl LogRecord {
    pub fn new(timestamp: i64, message: &str, record_id: String, key: String) -> Self {
        LogRecord {
            timestamp,
            message: message.to_owned(),
            record_id,
            key,
        }
    }
    pub fn into_parts(self) -> (i64, String, String, String) {
        (self.timestamp, self.message, self.record_id, self.key)
    }
}
impl From<(&String, &String)> for LogRecord {
    // This is used to parse the string from raw data
    fn from((raw_message, key): (&String, &String)) -> LogRecord {
        let record_id = uuid7().to_string();
        let mut split = raw_message.splitn(2, 'Z');
        let datetime_str = split.next().unwrap_or_else(|| {
            warn!("{}", LogParseError::MissingTimestamp(record_id.clone()));
            DEFAULT_TS
        });
        let ts = dt_from_ts(datetime_str)
            .map_err(|e| {
                warn!("{}", LogParseError::ParseError(record_id.clone(), e));
            })
            .unwrap_or(0);
        let message = split.next().unwrap_or(raw_message);

        LogRecord::new(ts, message, record_id, key.to_owned())
    }
}
impl TryFrom<ConsumerRecord> for LogRecord {
    type Error = serde_json::Error;

    // This is used for converting a fluvio ConsumerRecord into a LogRecord
    fn try_from(record: ConsumerRecord) -> Result<Self, Self::Error> {
        let payload = record.value();
        let data_str = String::from_utf8_lossy(payload);
        from_str::<LogRecord>(&data_str)
    }
}

pub fn dt_from_ts(ts: &str) -> Result<i64, ParseError> {
    match NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S"))
    {
        Ok(dt) => Ok(Utc.from_utc_datetime(&dt).timestamp_millis()),
        Err(e) => Err(e),
    }
}
