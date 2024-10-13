use crate::process::multipart::{into_multipart, process_metadata, process_stream};

use super::error::LogIntakeError;
use rocket::http::ContentType;
use rocket::post;
use rocket::Data;
use shared::connections::greptime::connect::GreptimeConnection;
use shared::connections::greptime::middleware::insert::logs_to_insert_request;
use shared::fluvio::FluvioConnection;
use shared::log_error;
use shared::router::auth::guard::AuthenticatedUser;
use shared::types::metadata::Metadata;
use std::ops::Deref;
use tracing::warn;

#[post("/logs", data = "<data>")]
pub async fn log_intake<'a>(
    user: AuthenticatedUser,
    greptime: GreptimeConnection,
    fluvio: FluvioConnection,
    content_type: &ContentType,
    data: Data<'a>,
) -> Result<String, LogIntakeError> {
    let mut multipart = into_multipart(content_type, data).await?;
    let mut metadata: Option<Metadata> = None;

    greptime.create_database(&user.db_name).await?;
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
                let mut logs = process_stream(field.data, &metadata)?;

                // insert to greptime
                let stream_inserter = greptime.streaming_inserter(&user.db_name)?;
                let insert_request = logs_to_insert_request(&logs, &metadata.pod_name);
                stream_inserter.insert(vec![insert_request]).await?;
                stream_inserter.finish().await?;

                // send to fluvio
                for log in logs.iter_mut() {
                    log.truncate_record(&user.customer_id, fluvio.topic.max_bytes);
                    let serialized_record = serde_json::to_string(&log).unwrap();
                    if serialized_record.len() > fluvio.topic.max_bytes {
                        warn!(
                            "Data too large for record, will be skipped. customer_id: {}, key: {}, record_id: {}, len: {}",
                            &user.customer_id,
                            log.key,
                            log.record_id,
                            serialized_record.len()
                        );
                        continue;
                    }
                    fluvio
                        .producer
                        .send(user.customer_id.clone(), serialized_record)
                        .await
                        .map_err(|e| log_error!(e))
                        .ok();
                    fluvio
                        .producer
                        .flush()
                        .await
                        .map_err(|e| log_error!(e))
                        .ok();
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
    use super::log_intake;
    use rocket::routes;
    use shared::connections::greptime::connect::GreptimeConnection;
    use shared::fluvio::{FluvioConnection, TopicName};
    use shared::mock::rocket::{rocket_test_client, TestClientError};
    use shared::router::rocket::Connection;
    use shared::tracing::setup::setup_tracing;
    use shared::utils::mock::mock_data::{get_test_data, TestCase};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};
    use test_case::test_case;

    #[tokio::test]
    #[test_case(TestCase::Simple)]
    #[test_case(TestCase::DataIntakeLimit)]
    async fn test_log_intake_route(case: TestCase) -> Result<(), TestClientError> {
        setup_tracing();
        // connections
        let greptime = GreptimeConnection::new().await?;
        let fluvio = FluvioConnection::new(TopicName::Log).await?;
        let connections: Vec<Connection> = vec![greptime.into(), fluvio.into()];

        // rocket client
        let client = rocket_test_client(&connections, routes![log_intake]).await?;

        // test data
        let test_data = get_test_data(case);
        let test_stream = get_multipart_stream(test_data);

        // test route
        let status = post_test_stream(&client, "/logs", test_stream).await;
        assert_eq!(status.code, 200);
        Ok(())
    }
}
