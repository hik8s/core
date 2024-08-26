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
use shared::connections::greptime::middleware::insert::logs_to_insert_request;
use shared::types::metadata::Metadata;
use std::io::Read;
use std::str::from_utf8;
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
                if metadata.is_none() {
                    error!("Metadata is not set");
                    return Err(Status::BadRequest);
                }
                let metadata = metadata.as_ref().unwrap();

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
                    let chunk = from_utf8(&buffer[..n]).map_err(|e| {
                        error!("Failed to convert buffer to UTF-8: {}", e);
                        Status::InternalServerError
                    })?;
                    logs.extend(process_chunk(chunk, &mut remainder, &metadata.pod_name));
                }
                let stream_inserter = db.greptime.streaming_inserter().map_err(|e| {
                    error!("Failed to get streaming inserter: {}", e);
                    Status::InternalServerError
                })?;

                let insert_request = logs_to_insert_request(&logs, &metadata.pod_name);

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
    use shared::tracing::setup::setup_tracing;
    use shared::utils::mock::mock_data::{get_test_data, TestCase};

    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};

    use crate::utils::mock::mock_server::rocket_test_client;

    #[rocket::async_test]
    async fn test_log_intake_route() {
        setup_tracing();
        let client = rocket_test_client().await.unwrap();

        let test_data = get_test_data(TestCase::Simple);
        let test_stream = get_multipart_stream(test_data);

        let status = post_test_stream(&client, "/logs", test_stream).await;
        assert_eq!(status.code, 200);
    }
}
