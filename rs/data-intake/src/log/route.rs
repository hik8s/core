use crate::process::multipart::{into_multipart, process_metadata, process_stream};

use super::error::LogIntakeError;
use rocket::http::ContentType;
use rocket::post;
use rocket::Data;
use shared::connections::greptime::connect::GreptimeConnection;
use shared::connections::greptime::middleware::insert::logs_to_insert_request;
use shared::fluvio::FluvioConnection;
use shared::router::auth::guard::AuthenticatedUser;
use shared::types::metadata::Metadata;
use std::ops::Deref;
use tracing::warn;

#[post("/logs", data = "<data>")]
pub async fn log_intake<'a>(
    _user: AuthenticatedUser,
    greptime_connection: GreptimeConnection,
    fluvio_connection: FluvioConnection,
    content_type: &ContentType,
    data: Data<'a>,
) -> Result<String, LogIntakeError> {
    let mut multipart = into_multipart(content_type, data).await?;
    let mut metadata: Option<Metadata> = None;

    // The loop will exit successfully if the stream field is processed,
    // exit with an error during processing and if no more field is found
    loop {
        // read multipart entry
        let field = multipart
            .read_entry()
            .map_err(|e| LogIntakeError::MultipartDataInvalid(e))?
            .ok_or_else(|| LogIntakeError::MultipartNoFields)?;
        match field.headers.name.deref() {
            "metadata" => {
                // process metadata
                process_metadata(field.data, &mut metadata)
                    .map_err(|e| LogIntakeError::MultipartMetadata(e))?;
            }
            "stream" => {
                // process stream
                let metadata = metadata.ok_or_else(|| LogIntakeError::MetadataNone)?;
                let logs = process_stream(field.data, &metadata)?;

                // insert to greptime
                let stream_inserter = greptime_connection.greptime.streaming_inserter()?;
                let insert_request = logs_to_insert_request(&logs, &metadata.pod_name);
                stream_inserter.insert(vec![insert_request]).await?;
                stream_inserter.finish().await?;

                // send to fluvio
                if let Err(e) = fluvio_connection.send_batch(&logs, &metadata).await {
                    let error = e.to_string();
                    for log in &logs {
                        let record_id = &log.record_id;
                        let key = &log.key;
                        let message_length = log.message.len();
                        warn!(
                            "Error: {}, Key: {}, Record ID: {}, Message Length: {}",
                            error, key, record_id, message_length
                        );
                    }
                }

                return Ok("Success".to_string());
            }
            field_name => {
                return Err(LogIntakeError::MultipartUnexpectedFieldName(
                    field_name.to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rocket::routes;
    use shared::connections::greptime::connect::GreptimeConnection;
    use shared::fluvio::{FluvioConnection, TopicName};
    use shared::mock::rocket::{rocket_test_client, TestClientError};
    use shared::router::rocket::Connection;
    use shared::tracing::setup::setup_tracing;
    use shared::utils::mock::mock_data::{get_test_data, TestCase};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};

    use super::log_intake;
    #[rocket::async_test]
    async fn test_log_intake_route() -> Result<(), TestClientError> {
        setup_tracing();
        // connections
        let greptime = GreptimeConnection::new().await?;
        let fluvio = FluvioConnection::new(TopicName::Log).await?;
        let connections: Vec<Connection> = vec![greptime.into(), fluvio.into()];

        // rocket client
        let client = rocket_test_client(&connections, routes![log_intake]).await?;

        // test data
        let test_data = get_test_data(TestCase::Simple);
        let test_stream = get_multipart_stream(test_data);

        // test route
        let status = post_test_stream(&client, "/logs", test_stream).await;
        assert_eq!(status.code, 200);
        Ok(())
    }
}
