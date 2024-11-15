use crate::error::DataIntakeError;

use super::chunk::process_chunk;
use multipart::server::{Multipart, MultipartData};
use rocket::data::ToByteUnit;
use rocket::http::ContentType;
use rocket::Data;
use serde_json::Value;
use shared::constant::DATA_INTAKE_LIMIT_MEMIBYTES;
use shared::log_error;
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
    metadata: &Metadata,
) -> Result<Vec<LogRecord>, MultipartStreamError> {
    let key: String = metadata.pod_name.to_owned();

    let mut buffer = Vec::new();
    let mut remainder = String::new();
    let mut logs = Vec::new();

    while let Ok(n) = data.read_to_end(&mut buffer) {
        if n == 0 {
            break;
        }
        let chunk = from_utf8(&buffer[..n]).map_err(|e| {
            error!("{e:?}, {key}");
            e
        })?;
        logs.extend(process_chunk(chunk, &mut remainder, metadata));
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
) -> Result<Multipart<Cursor<Vec<u8>>>, DataIntakeError> {
    let mut buffer = Vec::new();
    let limit = DATA_INTAKE_LIMIT_MEMIBYTES.mebibytes();
    let result = data.open(limit).stream_to(&mut buffer).await;
    match result {
        Ok(n) => {
            if !n.complete {
                return Err(log_error!(DataIntakeError::PayloadTooLarge(
                    limit.to_string()
                )));
            }
        }
        Err(e) => {
            return Err(DataIntakeError::MultipartIoError(log_error!(e)));
        }
    }

    let boundary = content_type
        .params()
        .find(|(k, _)| k == "boundary")
        .map(|(_, v)| v)
        .ok_or_else(|| DataIntakeError::ContentTypeBoundaryMissing)?;

    let multipart = Multipart::with_body(Cursor::new(buffer), boundary);
    Ok(multipart)
}
