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
    pub key: String,
    pub namespace: String,
    pub pod_uid: String,
    pub container: String,
}
impl ClassifiedLogRecord {
    pub fn new(log: &PreprocessedLogRecord, class: &Class) -> Self {
        ClassifiedLogRecord {
            timestamp: log.timestamp,
            message: log.message.to_owned(),
            record_id: log.record_id.to_owned(),
            preprocessed_message: log.preprocessed_message.join(" "),
            length: log.length,
            class_representation: class.to_string(),
            class_id: class.class_id.to_owned(),
            similarity: class.similarity,
            key: log.key.to_owned(),
            namespace: log.namespace.to_owned(),
            pod_uid: log.pod_uid.to_owned(),
            container: log.container.to_owned(),
        }
    }
}
