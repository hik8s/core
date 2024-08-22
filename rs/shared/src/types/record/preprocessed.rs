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
        preprocessed_message: Vec<String>,
        record_id: String,
    ) -> Self {
        PreprocessedLogRecord {
            timestamp,
            message,
            length: preprocessed_message.len() as u64,
            preprocessed_message,
            record_id,
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

impl From<(LogRecord, Vec<String>)> for PreprocessedLogRecord {
    fn from((log, preprocessed_message): (LogRecord, Vec<String>)) -> Self {
        let (timestamp, record_id, message) = log.into_parts();
        PreprocessedLogRecord::new(timestamp, message, preprocessed_message, record_id)
    }
}
