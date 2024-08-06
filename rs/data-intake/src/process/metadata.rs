use std::io::Read;

use multipart::server::{Multipart, MultipartData};
use serde_json::Value;
use shared::types::metadata::Metadata;
use std::error::Error;
use std::io::Cursor;

pub fn process_metadata(
    mut data: MultipartData<&mut Multipart<Cursor<Vec<u8>>>>,
    metadata: &mut Option<Metadata>,
) -> Result<(), Box<dyn Error>> {
    let mut text = String::new();
    data.read_to_string(&mut text)?;
    let metadata_json: Value = serde_json::from_str(&text)?;

    // Extract path from metadata
    if let Some(path) = metadata_json.get("path").and_then(Value::as_str) {
        if let Some(filename) = metadata_json.get("file").and_then(Value::as_str) {
            match Metadata::from_path(filename, path) {
                Ok(meta) => *metadata = Some(meta),
                Err(e) => {
                    tracing::error!("Failed to create metadata from path: {}", e);
                    return Err(e);
                }
            }
        } else {
            tracing::error!("File is missing in the multipart metadata");
            return Err("File is missing in the multipart metadata".into());
        }
    } else {
        tracing::error!("Path is missing in the multipart metadata");
        return Err("Path is missing in the multipart metadata".into());
    }
    Ok(())
}
