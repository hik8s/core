use super::log::LogParseError;
use fluvio::dataplane::record::ConsumerRecord;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConsumerRecordError {
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("LogRecord error: {1}, key: {0}, record_id could not be deserialized")]
    LogParseError(String, LogParseError),
    #[error("Missing key in ConsumerRecord")]
    MissingKey,
    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
}

pub fn get_payload_and_key(
    record: ConsumerRecord,
) -> Result<(Vec<u8>, String), ConsumerRecordError> {
    let payload = record.value().to_vec();
    let key: Vec<u8> = record
        .key()
        .map(|k| k.to_vec())
        .ok_or(ConsumerRecordError::MissingKey)?;
    let key_str = String::from_utf8(key)?;

    Ok((payload, key_str))
}
