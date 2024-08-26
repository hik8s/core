use crate::preprocessing::log::preprocess_log;

use super::log::LogRecord;

#[derive(Debug, Clone)]
pub struct PreprocessedLogRecord {
    pub timestamp: i64,
    pub message: String,
    pub record_id: String,
    pub preprocessed_message: Vec<String>,
    pub length: u64,
    pub key: String,
}

impl PreprocessedLogRecord {
    pub fn new(
        timestamp: i64,
        message: String,
        record_id: String,
        preprocessed_message: Vec<String>,
        key: String,
    ) -> Self {
        PreprocessedLogRecord {
            timestamp,
            message,
            record_id,
            length: preprocessed_message.len() as u64,
            preprocessed_message,
            key,
        }
    }

    pub fn into_parts(&self) -> (i64, String, String, Vec<String>, u64, String) {
        (
            self.timestamp,
            self.message.to_owned(),
            self.record_id.to_owned(),
            self.preprocessed_message.to_owned(),
            self.length,
            self.key.to_owned(),
        )
    }
}

impl From<(&String, &String)> for PreprocessedLogRecord {
    fn from((raw_message, key): (&String, &String)) -> Self {
        let log = LogRecord::from((raw_message, key));
        PreprocessedLogRecord::from(log)
    }
}

impl From<LogRecord> for PreprocessedLogRecord {
    fn from(log: LogRecord) -> Self {
        let preprocessed_message = preprocess_log(&log);
        PreprocessedLogRecord {
            timestamp: log.timestamp,
            message: log.message,
            record_id: log.record_id,
            key: log.key,
            length: preprocessed_message.len() as u64,
            preprocessed_message,
        }
    }
}
