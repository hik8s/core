use crate::types::classification::class::Class;

use super::preprocessed::PreprocessedLogRecord;

pub struct ClassifiedLogRecord {
    pub timestamp: i64,
    pub message: String,
    pub record_id: String,
    pub preprocessed_message: String,
    pub length: u64,
    pub class_representation: String,
    pub class_id: String,
    pub similarity: f64,
}
impl ClassifiedLogRecord {
    pub fn new(log: PreprocessedLogRecord, class: Class) -> Self {
        let (timestamp, message, record_id, preprocessed_message, length) = log.into_parts();
        ClassifiedLogRecord {
            timestamp,
            message,
            record_id,
            preprocessed_message: preprocessed_message.join(" "),
            length,
            class_representation: class.to_string(),
            class_id: class.class_id,
            similarity: class.similarity,
        }
    }
}
