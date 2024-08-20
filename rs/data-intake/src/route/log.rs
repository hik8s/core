use crate::middleware::greptime::insert::to_insert_request;
use crate::process::log::process_chunk;
use crate::process::metadata::process_metadata;
use multipart::server::Multipart;
use rocket::data::ToByteUnit;
use rocket::http::ContentType;
use rocket::http::Status;
use rocket::post;
use rocket::Data;
use shared::connections::fluvio::connect::FluvioConnection;
use shared::connections::greptime::connect::GreptimeConnection;
use shared::types::metadata::Metadata;
use std::io::Read;
use std::{io::Cursor, ops::Deref};
use tracing::{error, info, warn};

#[post("/logs", data = "<data>")]
pub async fn log_intake<'a>(
    db: GreptimeConnection,
    fluvio_connection: FluvioConnection,
    content_type: &ContentType,
    data: Data<'a>,
) -> Result<String, Status> {
    let mut d = Vec::new();
    data.open(100_u64.mebibytes())
        .stream_to(&mut d)
        .await
        .map_err(|e| {
            error!("Failed to stream data: {}", e);
            Status::PayloadTooLarge
        })?;

    let boundary = match content_type
        .params()
        .find(|(k, _)| k == "boundary")
        .map(|(_, v)| v)
    {
        Some(b) => b,
        None => {
            error!("Missing boundary in content type");
            return Err(Status::BadRequest);
        }
    };

    let mut multipart = Multipart::with_body(Cursor::new(d.clone()), boundary);

    // Check if the multipart data is valid
    multipart.read_entry().map_err(|e| {
        error!("Invalid multipart data: {}", e);
        Status::BadRequest
    })?;

    // Initialize Metadata as None
    let mut metadata: Option<Metadata> = None;

    // Create a new Multipart object
    let mut multipart = Multipart::with_body(Cursor::new(d), boundary);

    while let Ok(Some(mut field)) = multipart.read_entry() {
        match field.headers.name.deref() {
            "metadata" => {
                let data = field.data;

                process_metadata(data, &mut metadata).map_err(|e| {
                    error!("Metadata processing error: {}", e);
                    Status::InternalServerError
                })?;
            }
            "stream" => {
                let mut buffer = [0; 262144];
                let mut remainder = String::new();
                let mut logs = Vec::new();

                while let Ok(n) = field.data.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    if n == buffer.len() {
                        warn!("Buffer is full");
                    }
                    let chunk = std::str::from_utf8(&buffer[..n]).map_err(|e| {
                        error!("Failed to convert buffer to UTF-8: {}", e);
                        Status::InternalServerError
                    })?;
                    logs.extend(process_chunk(chunk, &mut remainder));
                }
                match metadata {
                    Some(ref metadata) => {
                        let stream_inserter = db.greptime.streaming_inserter().map_err(|e| {
                            error!("Failed to get streaming inserter: {}", e);
                            Status::InternalServerError
                        })?;

                        let insert_request = to_insert_request(&logs, metadata);

                        if let Err(e) = stream_inserter.insert(vec![insert_request]).await {
                            error!("Error during insert: {}", e);
                            return Err(Status::InternalServerError);
                        }

                        match stream_inserter.finish().await {
                            Ok(rows) => {
                                info!("Rows written: {}", rows);
                            }
                            Err(e) => {
                                error!("Error during finish: {}", e);
                                return Err(Status::InternalServerError);
                            }
                        }
                        fluvio_connection
                            .send_batch(logs, metadata)
                            .await
                            .map_err(|e| {
                                error!("Failed to send batch to Fluvio topic: {}", e);
                                Status::InternalServerError
                            })?;

                        return Ok("Success".to_string());
                    }
                    None => {
                        error!("Metadata is not set");
                        return Err(Status::BadRequest);
                    }
                }
            }
            _ => {
                error!("Unexpected field name: {}", field.headers.name);
                return Err(Status::BadRequest);
            }
        }
    }
    error!("Expected field name: metadata or stream. No data was received.");
    Err(Status::BadRequest)
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
    async fn test_log_intake_route() {
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
        let status = post_test_stream(&client, "/logs", test_stream).await;
        assert_eq!(status.code, 200);
    }
}
