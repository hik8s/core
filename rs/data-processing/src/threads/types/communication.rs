use fluvio::dataplane::record::ConsumerRecord;
use shared::types::record::log::{LogParseError, LogRecord};
use std::convert::TryInto;
use thiserror::Error;

pub struct ClassificationTask {
    pub log: LogRecord,
    pub key: String,
}

pub struct ClassificationResult {
    pub key: String,
    pub log_id: String,
    pub class_id: String,
}

impl ClassificationResult {
    pub fn new(key: &String, record_id: &String, class_id: &String) -> Self {
        ClassificationResult {
            key: key.to_owned(),
            class_id: class_id.to_owned(),
            log_id: record_id.to_owned(),
        }
    }
}

impl From<(LogRecord, String)> for ClassificationTask {
    fn from((log, key): (LogRecord, String)) -> Self {
        ClassificationTask { log, key }
    }
}

#[derive(Debug, Error)]
pub enum ConsumerRecordError {
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("LogRecord error: {1}, key: {0}, record_id could not be deserialized")]
    LogRecordError(String, LogParseError),
    #[error("Missing key in ConsumerRecord")]
    MissingKey,
}

impl TryFrom<ConsumerRecord> for ClassificationTask {
    type Error = ConsumerRecordError;

    fn try_from(record: ConsumerRecord) -> Result<Self, Self::Error> {
        let payload = record.value().to_vec();
        let key = record
            .key()
            .map(|k| k.to_vec())
            .ok_or(ConsumerRecordError::MissingKey)?;
        let key_str = String::from_utf8(key)?;

        let log: LogRecord = payload
            .try_into()
            .map_err(|e| ConsumerRecordError::LogRecordError(key_str.clone(), e))?;

        Ok(ClassificationTask { log, key: key_str })
    }
}
