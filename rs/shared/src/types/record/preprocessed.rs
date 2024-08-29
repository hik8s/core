use crate::{preprocessing::log::preprocess_message, types::metadata::Metadata};

use super::log::LogRecord;

#[derive(Debug, Clone)]
pub struct PreprocessedLogRecord {
    pub timestamp: i64,
    pub message: String,
    pub record_id: String,
    pub preprocessed_message: Vec<String>,
    pub length: u64,
    pub key: String,
    pub namespace: String,
    pub pod_uid: String,
    pub container: String,
}

impl PreprocessedLogRecord {
    pub fn new(
        timestamp: i64,
        message: String,
        record_id: String,
        preprocessed_message: Vec<String>,
        key: String,
        namespace: String,
        pod_uid: String,
        container: String,
    ) -> Self {
        PreprocessedLogRecord {
            timestamp,
            message,
            record_id,
            length: preprocessed_message.len() as u64,
            preprocessed_message,
            key,
            namespace,
            pod_uid,
            container,
        }
    }
}

impl From<(&String, &Metadata)> for PreprocessedLogRecord {
    fn from((raw_message, metadata): (&String, &Metadata)) -> Self {
        let log = LogRecord::from((raw_message, metadata));
        PreprocessedLogRecord::from(log)
    }
}

impl From<LogRecord> for PreprocessedLogRecord {
    fn from(log: LogRecord) -> Self {
        let preprocessed_message = preprocess_message(&log.message);
        PreprocessedLogRecord {
            timestamp: log.timestamp,
            message: log.message,
            record_id: log.record_id,
            length: preprocessed_message.len() as u64,
            preprocessed_message,
            key: log.key,
            namespace: log.namespace,
            pod_uid: log.pod_uid,
            container: log.container,
        }
    }
}
