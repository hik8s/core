use fluvio::dataplane::record::ConsumerRecord;
use shared::types::record::{
    classified::ClassifiedLogRecord,
    consumer_record::{get_payload_and_key, ConsumerRecordError},
    log::LogRecord,
};
use std::convert::TryInto;

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
    pub fn new(key: &String, log: &ClassifiedLogRecord) -> Self {
        ClassificationResult {
            key: key.to_owned(),
            class_id: log.class_id.to_owned(),
            log_id: log.record_id.to_owned(),
        }
    }
}

impl From<(LogRecord, String)> for ClassificationTask {
    fn from((log, key): (LogRecord, String)) -> Self {
        ClassificationTask { log, key }
    }
}

impl TryFrom<ConsumerRecord> for ClassificationTask {
    type Error = ConsumerRecordError;

    fn try_from(record: ConsumerRecord) -> Result<Self, Self::Error> {
        let (payload, key) = get_payload_and_key(record)?;

        let log: LogRecord = payload.try_into()?;

        Ok(ClassificationTask { log, key })
    }
}
