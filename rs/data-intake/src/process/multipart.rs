use crate::log::error::LogIntakeError;

use super::chunk::process_chunk;
use multipart::server::{Multipart, MultipartData};
use rocket::data::ToByteUnit;
use rocket::http::ContentType;
use rocket::Data;
use serde_json::Value;
use shared::types::metadata::Metadata;
use shared::types::record::log::LogRecord;
use std::io::Cursor;
use std::io::Read;
use std::str::from_utf8;
use std::str::Utf8Error;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum MultipartStreamError {
    #[error("Failed to read data: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to convert buffer to UTF-8: {0}")]
    Utf8ConversionError(#[from] Utf8Error),
    #[error("Buffer is full for key: {0}")]
    BufferFull(String),
}

pub fn process_stream(
    mut data: MultipartData<&mut Multipart<Cursor<Vec<u8>>>>,
    key: &String,
) -> Result<Vec<LogRecord>, MultipartStreamError> {
    let mut buffer = [0; 262144];
    let mut remainder = String::new();
    let mut logs = Vec::new();

    while let Ok(n) = data.read(&mut buffer) {
        if n == 0 {
            break;
        }
        if n == buffer.len() {
            return {
                error!("Buffer full for key: {}", key);
                Err(MultipartStreamError::BufferFull(key.clone()))
            };
        }
        let chunk = from_utf8(&buffer[..n])?;
        logs.extend(process_chunk(chunk, &mut remainder, &key));
    }

    Ok(logs)
}

#[derive(Error, Debug)]
pub enum MultipartMetadataError {
    #[error("Failed to read data: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("Path is missing in the multipart metadata")]
    MissingPath,
    #[error("File is missing in the multipart metadata")]
    MissingFile,
    #[error("Failed to create metadata from path: {0}")]
    FailedProcessingPath(String),
}

pub fn process_metadata(
    mut data: MultipartData<&mut Multipart<Cursor<Vec<u8>>>>,
    metadata: &mut Option<Metadata>,
) -> Result<(), MultipartMetadataError> {
    let mut text = String::new();
    data.read_to_string(&mut text)?;
    let metadata_json: Value = serde_json::from_str(&text)?;

    // Extract path from metadata
    if let Some(path) = metadata_json.get("path").and_then(Value::as_str) {
        if let Some(filename) = metadata_json.get("file").and_then(Value::as_str) {
            match Metadata::from_path(filename, path) {
                Ok(meta) => *metadata = Some(meta),
                Err(e) => {
                    return Err(MultipartMetadataError::FailedProcessingPath(e.to_string()));
                }
            }
        } else {
            return Err(MultipartMetadataError::MissingFile);
        }
    } else {
        return Err(MultipartMetadataError::MissingPath);
    }
    Ok(())
}

pub async fn into_multipart<'a>(
    content_type: &ContentType,
    data: Data<'a>,
) -> Result<Multipart<Cursor<Vec<u8>>>, LogIntakeError> {
    let mut buffer = Vec::new();
    data.open(100_u64.mebibytes())
        .stream_to(&mut buffer)
        .await
        .map_err(|e| LogIntakeError::PayloadTooLarge(e))?;

    let boundary = content_type
        .params()
        .find(|(k, _)| k == "boundary")
        .map(|(_, v)| v)
        .ok_or_else(|| LogIntakeError::ContentTypeBoundaryMissing)?;

    let multipart = Multipart::with_body(Cursor::new(buffer), boundary);
    Ok(multipart)
}
