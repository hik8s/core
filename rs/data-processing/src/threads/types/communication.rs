use fluvio::dataplane::record::ConsumerRecord;
use shared::types::record::log::{LogRecord, LogRecordError};
use std::convert::TryInto;
use thiserror::Error;

pub struct ClassificationTask {
    pub log_record: LogRecord,
    pub key: String,
}

pub struct ClassificationResult {
    pub key: String,
    pub log_id: String,
    pub class_id: String,
}

impl ClassificationResult {
    pub fn new(task: &ClassificationTask, class_id: String) -> Self {
        ClassificationResult {
            key: task.key.to_owned(),
            class_id,
            log_id: task.log_record.record_id.to_owned(),
        }
    }
}

impl From<(LogRecord, String)> for ClassificationTask {
    fn from((log_record, key): (LogRecord, String)) -> Self {
        ClassificationTask { log_record, key }
    }
}

#[derive(Debug, Error)]
pub enum ConsumerRecordError {
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("LogRecord error: {source}, key: {key}, record_id could not be deserialized")]
    LogRecordError { source: LogRecordError, key: String },
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

        let log_record: LogRecord =
            payload
                .try_into()
                .map_err(|e| ConsumerRecordError::LogRecordError {
                    source: e,
                    key: key_str.clone(),
                })?;

        Ok(ClassificationTask {
            log_record,
            key: key_str,
        })
    }
}
