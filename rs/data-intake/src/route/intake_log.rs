use crate::process::multipart::{into_multipart, process_metadata, process_stream};

use crate::error::DataIntakeError;
use rocket::http::ContentType;
use rocket::post;
use rocket::Data;
use shared::connections::greptime::greptime_connection::GreptimeTable;
use shared::connections::greptime::middleware::insert::logs_to_insert_request;
use shared::fluvio::TopicName;
use shared::log_error;
use shared::router::auth::guard::AuthenticatedUser;
use shared::types::metadata::Metadata;
use shared::DbName;
use shared::FluvioConnection;
use shared::GreptimeConnection;
use std::ops::Deref;
use tracing::warn;

#[post("/logs", data = "<data>")]
pub async fn log_intake<'a>(
    user: AuthenticatedUser,
    greptime: GreptimeConnection,
    fluvio: FluvioConnection,
    content_type: &ContentType,
    data: Data<'a>,
) -> Result<String, DataIntakeError> {
    let mut multipart = into_multipart(content_type, data).await?;
    let mut metadata: Option<Metadata> = None;
    let topic = TopicName::Log;
    let db = DbName::Log.id(&user.customer_id);

    greptime.create_database(&db).await?;
    // The loop will exit successfully if the stream field is processed,
    // exit with an error during processing and if no more field is found
    loop {
        // read multipart entry
        let field = multipart
            .read_entry()
            .map_err(DataIntakeError::MultipartDataInvalid)?
            .ok_or_else(|| DataIntakeError::MultipartNoFields)?;
        match field.headers.name.deref() {
            "metadata" => {
                // process metadata
                process_metadata(field.data, &mut metadata)
                    .map_err(DataIntakeError::MultipartMetadata)?;
            }
            "stream" => {
                // process stream
                let metadata = metadata.ok_or(DataIntakeError::MetadataNone)?;
                let mut logs = process_stream(field.data, &metadata)?;

                // insert to greptime
                let table = GreptimeTable::from(&metadata);
                let stream_inserter = greptime.streaming_inserter(&db)?;
                let insert_request = logs_to_insert_request(&logs, table);
                stream_inserter.insert(vec![insert_request]).await?;
                stream_inserter.finish().await?;

                // send to fluvio
                for log in logs.iter_mut() {
                    let max_bytes = fluvio.get_topic(topic).max_bytes;
                    log.truncate_record(&db, max_bytes);
                    let serialized_record = serde_json::to_string(&log).unwrap();
                    if serialized_record.len() > max_bytes {
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
                        .get_producer(topic)
                        .send(user.customer_id.clone(), serialized_record)
                        .await
                        .map_err(|e| log_error!(e))
                        .ok();
                    fluvio
                        .get_producer(topic)
                        .flush()
                        .await
                        .map_err(|e| log_error!(e))
                        .ok();
                }
                return Ok("Success".to_string());
            }
            field_name => {
                return Err(DataIntakeError::MultipartUnexpectedFieldName(
                    field_name.to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DataIntakeError;
    use crate::server::initialize_data_intake;

    use rstest::rstest;
    use shared::mock::rocket::get_test_client;

    use shared::setup_tracing;
    use shared::utils::mock::mock_data::{get_test_data, TestCase};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};

    #[tokio::test]
    #[rstest]
    #[case(TestCase::Simple)]
    #[case(TestCase::DataIntakeLimit)]
    async fn test_log_intake_route(#[case] case: TestCase) -> Result<(), DataIntakeError> {
        setup_tracing(false);

        // rocket client
        let server = initialize_data_intake().await?;
        let client = get_test_client(server).await?;

        // test data
        let test_data = get_test_data(case);
        let test_stream = get_multipart_stream(&test_data);

        // test route
        let status = post_test_stream(&client, "/logs", test_stream).await;
        assert_eq!(status.code, 200);
        Ok(())
    }
}
