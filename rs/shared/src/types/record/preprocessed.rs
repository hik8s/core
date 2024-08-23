use crate::preprocessing::log::preprocess_log;

use super::log::LogRecord;

#[derive(Debug)]
pub struct PreprocessedLogRecord {
    pub timestamp: i64,
    pub message: String,
    pub record_id: String,
    pub preprocessed_message: Vec<String>,
    pub length: u64,
}

impl PreprocessedLogRecord {
    pub fn new(
        timestamp: i64,
        message: String,
        record_id: String,
        preprocessed_message: Vec<String>,
    ) -> Self {
        PreprocessedLogRecord {
            timestamp,
            message,
            record_id,
            length: preprocessed_message.len() as u64,
            preprocessed_message,
        }
    }

    pub fn into_parts(self) -> (i64, String, String, Vec<String>, u64) {
        (
            self.timestamp,
            self.message,
            self.record_id,
            self.preprocessed_message,
            self.length,
        )
    }
}

impl From<String> for PreprocessedLogRecord {
    fn from(raw_message: String) -> Self {
        let log = LogRecord::from(&raw_message);
        preprocess_log(log)
    }
}
