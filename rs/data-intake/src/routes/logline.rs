use multipart::server::Multipart;
use rocket::data::ToByteUnit;
use rocket::http::ContentType;
use rocket::post;
use rocket::Data;
use shared::db::greptime::connect::GreptimeConnection;
use shared::types::metadata::Metadata;
use std::io::Read;
use std::{io::Cursor, ops::Deref};

use crate::middleware::greptime::insert::to_insert_request;
use crate::process::logline::process_chunk;
use crate::process::metadata::process_metadata;

#[post("/class", data = "<data>")]
pub async fn logline<'a>(
    db: GreptimeConnection,
    content_type: &ContentType,
    data: Data<'a>,
) -> Result<String, std::io::Error> {
    let mut d = Vec::new();
    data.open(100_u64.mebibytes()).stream_to(&mut d).await?;

    let boundary = match content_type
        .params()
        .find(|(k, _)| k == "boundary")
        .map(|(_, v)| v)
    {
        Some(b) => b,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Missing boundary in content type",
            ))
        }
    };

    let mut multipart = Multipart::with_body(Cursor::new(d.clone()), boundary);

    // Check if the multipart data is valid
    if let Err(e) = multipart.read_entry() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid multipart data: {}", e),
        ));
    }

    // Create a new Multipart object
    let mut multipart = Multipart::with_body(Cursor::new(d), boundary);

    // Initialize Metadata as None
    let mut metadata: Option<Metadata> = None;

    while let Ok(Some(mut field)) = multipart.read_entry() {
        match field.headers.name.deref() {
            "metadata" => {
                let data = field.data;

                if let Err(e) = process_metadata(data, &mut metadata) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("metadata processing error: {}", e),
                    ));
                }
            }
            "stream" => {
                let mut buffer = [0; 262144];
                let mut remainder = String::new();
                let mut lines = Vec::new();

                while let Ok(n) = field.data.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    if n == buffer.len() {
                        tracing::warn!("Buffer is full");
                    }
                    let chunk = std::str::from_utf8(&buffer[..n])
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    lines.extend(process_chunk(chunk, &mut remainder));
                }
                match metadata {
                    Some(ref metadata) => {
                        let stream_inserter = db.greptime.streaming_inserter().unwrap();

                        let insert_request = to_insert_request(lines, metadata);

                        if let Err(e) = stream_inserter.insert(vec![insert_request]).await {
                            tracing::error!("Error: {e}");
                        }
                        match stream_inserter.finish().await {
                            Ok(rows) => {
                                tracing::info!("Rows written: {rows}");
                            }
                            Err(e) => {
                                tracing::info!("Error: {e}");
                                panic!("Error: {e}")
                            }
                        }

                        // locks apps vector only for getting app
                        tracing::info!("Processed metadata for {}", metadata.filename);

                        return Ok("success".to_string());
                    }
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "metadata is not set",
                        ));
                    }
                }
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unexpected field name: {}", field.headers.name),
                ))
            }
        }
    }
    return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Expected field name: metadata or stream. No data was received."),
    ));
}

#[cfg(test)]
mod tests {
    use shared::{tracing::setup::setup_tracing, types::metadata::Metadata};

    use crate::test_utils::{
        mock_client::post_test_stream,
        mock_data::{generate_random_filename, get_test_path},
        mock_server::rocket_test_client,
        mock_stream::get_multipart_stream,
    };

    #[rocket::async_test]
    async fn test_loglines_route() {
        setup_tracing();
        let client = rocket_test_client().await;

        let test_data = [
            "2023-06-10T10:30:01Z INFO This is a test log line 1\r".to_string(),
            "2023-06-10T10:30:02Z INFO This is a test log line 2\r".to_string(),
            "2023-06-10T10:30:03Z INFO This is a test log line 3\r".to_string(),
            "2023-06-10T10:30:04Z INFO This is a test log line 4\r".to_string(),
        ];
        let test_metadata =
            &Metadata::from_path(&generate_random_filename(), &get_test_path()).unwrap();
        let test_stream =
            get_multipart_stream(&test_metadata.filename, &test_metadata.path, &test_data);
        let status = post_test_stream(&client, "/class", test_stream).await;
        assert_eq!(status.code, 200);
    }
}
