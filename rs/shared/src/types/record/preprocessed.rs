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

impl From<(LogRecord, Vec<String>)> for PreprocessedLogRecord {
    fn from((log, preprocessed_message): (LogRecord, Vec<String>)) -> Self {
        let (timestamp, message, record_id) = log.into_parts();
        PreprocessedLogRecord::new(timestamp, message, record_id, preprocessed_message)
    }
}
