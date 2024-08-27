use crate::process::metadata::process_metadata;
use crate::process::multipart::process_multipart_field;
use multipart::server::Multipart;
use rocket::data::ToByteUnit;
use rocket::http::ContentType;
use rocket::post;
use rocket::Data;
use shared::connections::fluvio::connect::FluvioConnection;
use shared::connections::greptime::connect::GreptimeConnection;
use shared::connections::greptime::middleware::insert::logs_to_insert_request;
use shared::types::metadata::Metadata;
use std::{io::Cursor, ops::Deref};

use super::error::LogIntakeError;

#[post("/logs", data = "<data>")]
pub async fn log_intake<'a>(
    db: GreptimeConnection,
    fluvio_connection: FluvioConnection,
    content_type: &ContentType,
    data: Data<'a>,
) -> Result<String, LogIntakeError> {
    let mut d = Vec::new();
    data.open(100_u64.mebibytes())
        .stream_to(&mut d)
        .await
        .map_err(|e| LogIntakeError::StreamDataError(e))?;

    let boundary = match content_type
        .params()
        .find(|(k, _)| k == "boundary")
        .map(|(_, v)| v)
    {
        Some(b) => b,
        None => return Err(LogIntakeError::MissingBoundary),
    };

    let mut multipart = Multipart::with_body(Cursor::new(d.clone()), boundary);

    // Check if the multipart data is valid
    multipart
        .read_entry()
        .map_err(|e| LogIntakeError::InvalidMultipartData(e))?;

    // Initialize Metadata as None
    let mut metadata: Option<Metadata> = None;

    // Create a new Multipart object
    let mut multipart = Multipart::with_body(Cursor::new(d), boundary);

    while let Ok(Some(mut field)) = multipart.read_entry() {
        match field.headers.name.deref() {
            "metadata" => {
                let data = field.data;

                process_metadata(data, &mut metadata)
                    .map_err(|e| LogIntakeError::MetadataProcessingError(e))?;
            }
            "stream" => {
                if metadata.is_none() {
                    return Err(LogIntakeError::MetadataNotSet);
                }
                let metadata = metadata.as_ref().unwrap();

                let logs = process_multipart_field(&mut field, &metadata.pod_name)?;
                let stream_inserter = db
                    .greptime
                    .streaming_inserter()
                    .map_err(|e| LogIntakeError::GreptimeIngestError(e))?;

                let insert_request = logs_to_insert_request(&logs, &metadata.pod_name);

                if let Err(e) = stream_inserter.insert(vec![insert_request]).await {
                    return Err(LogIntakeError::GreptimeIngestError(e));
                }

                stream_inserter
                    .finish()
                    .await
                    .map_err(|e| LogIntakeError::GreptimeIngestError(e))?;

                fluvio_connection
                    .send_batch(logs, metadata)
                    .await
                    .map_err(|e| LogIntakeError::FluvioConnectionError(e))?;

                return Ok("Success".to_string());
            }
            field_name => {
                return Err(LogIntakeError::UnexpectedFieldName(field_name.to_string()));
            }
        }
    }
    Err(LogIntakeError::NoDataReceived)
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
