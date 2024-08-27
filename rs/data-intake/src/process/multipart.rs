use multipart::server::{Multipart, MultipartField};
use shared::types::record::log::LogRecord;
use std::io::Cursor;
use std::io::Read;
use std::str::from_utf8;
use std::str::Utf8Error;
use thiserror::Error;
use tracing::error;

use super::chunk::process_chunk;

#[derive(Error, Debug)]
pub enum MultipartProcessError {
    #[error("Failed to read data: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to convert buffer to UTF-8: {0}")]
    Utf8ConversionError(#[from] Utf8Error),
    #[error("Buffer is full for key: {0}")]
    BufferFull(String),
}

pub fn process_multipart_field(
    field: &mut MultipartField<&mut Multipart<Cursor<Vec<u8>>>>,
    key: &String,
) -> Result<Vec<LogRecord>, MultipartProcessError> {
    let mut buffer = [0; 262144];
    let mut remainder = String::new();
    let mut logs = Vec::new();

    while let Ok(n) = field.data.read(&mut buffer) {
        if n == 0 {
            break;
        }
        if n == buffer.len() {
            return {
                error!("Buffer full for key: {}", key);
                Err(MultipartProcessError::BufferFull(key.clone()))
            };
        }
        let chunk = from_utf8(&buffer[..n])?;
        logs.extend(process_chunk(chunk, &mut remainder, &key));
    }

    Ok(logs)
}
