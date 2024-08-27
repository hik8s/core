use multipart::server::{Multipart, MultipartData};
use serde_json::Value;
use shared::types::metadata::Metadata;
use std::io::Cursor;
use std::io::Read;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetadataError {
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
) -> Result<(), MetadataError> {
    let mut text = String::new();
    data.read_to_string(&mut text)?;
    let metadata_json: Value = serde_json::from_str(&text)?;

    // Extract path from metadata
    if let Some(path) = metadata_json.get("path").and_then(Value::as_str) {
        if let Some(filename) = metadata_json.get("file").and_then(Value::as_str) {
            match Metadata::from_path(filename, path) {
                Ok(meta) => *metadata = Some(meta),
                Err(e) => {
                    return Err(MetadataError::FailedProcessingPath(e.to_string()));
                }
            }
        } else {
            return Err(MetadataError::MissingFile);
        }
    } else {
        return Err(MetadataError::MissingPath);
    }
    Ok(())
}
