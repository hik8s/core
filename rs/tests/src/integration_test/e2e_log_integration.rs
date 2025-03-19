#[cfg(test)]
mod tests {
    use data_intake::error::DataIntakeError;
    use data_intake::server::initialize_data_intake;
    use data_processing::run::run_log_processing;
    use data_vectorizer::vectorize_class;

    use rstest::rstest;
    use shared::connections::greptime::greptime_connection::GreptimeTable;
    use shared::constant::OPENAI_EMBEDDING_TOKEN_LIMIT;
    use shared::mock::rocket::get_test_client;
    use shared::qdrant_util::string_filter;
    use shared::setup_tracing;
    use shared::types::class::vectorized::{from_scored_point, VectorizedClass};

    use shared::utils::mock::mock_data::{get_test_data, TestCase};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};
    use shared::DbName;
    use shared::GreptimeConnection;
    use shared::RateLimiter;
    use shared::{get_env_var, QdrantConnection};
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex, Once};
    use std::time::Duration;
    use tracing::info;

    use tokio::time::sleep;

    static THREAD_LOG_PROCESSING: Once = Once::new();

    lazy_static::lazy_static! {
        static ref RECEIVED_LOGS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    }
    #[tokio::test]
    #[rstest]
    #[case(TestCase::Simple)]
    #[case(TestCase::DataIntakeLimit)]
    #[case(TestCase::DataProcessingLimit)]
    // #[case(TestCase::OpenAiRateLimit)]
    async fn e2e_log_integration(#[case] case: TestCase) -> Result<(), DataIntakeError> {
        setup_tracing(true);
        let num_cases = 3;
        let test_data = get_test_data(case);
        // data intake
        let server = initialize_data_intake().await.unwrap();
        let client = get_test_client(server).await?;

        // ingest data
        let test_stream = get_multipart_stream(&test_data);
        let status = post_test_stream(&client, "/logs", test_stream).await;
        assert_eq!(status.code, 200);

        // data processing
        THREAD_LOG_PROCESSING.call_once(|| {
            run_log_processing().unwrap();

            // data vectorizer
            tokio::spawn(async move {
                let limiter = Arc::new(RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT));
                vectorize_class(limiter).await.unwrap();
            });
        });

        let greptime = GreptimeConnection::new().await?;
        let qdrant = QdrantConnection::new().await.unwrap();
        let table = GreptimeTable::from(&test_data.metadata);
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let db = DbName::Log.id(&customer_id);
        qdrant.create_collection(&db).await.unwrap();

        let start_time = tokio::time::Instant::now();
        let timeout = Duration::from_secs(30);
        let mut rows = Vec::new();
        let mut classes: Vec<VectorizedClass> = Vec::new();

        while start_time.elapsed() < timeout {
            // check greptime
            rows = greptime
                .query(&db, &table.format_name(), "record_id")
                .await
                .unwrap();

            // check qdrant
            let filter = string_filter("key", &test_data.metadata.pod_name);
            let points = qdrant
                .query_points(&db, Some(filter), 1000, true)
                .await
                .unwrap();
            classes = from_scored_point(points).unwrap();
            info!(
                "Classes: {}/{} | Pod: {}",
                classes.len(),
                test_data.expected_class.count,
                table.format_name()
            );
            if !rows.is_empty() && classes.len() == test_data.expected_class.count as usize {
                // successfully received data
                RECEIVED_LOGS
                    .lock()
                    .unwrap()
                    .insert(table.format_name().clone());
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }

        while RECEIVED_LOGS.lock().unwrap().len() < num_cases && start_time.elapsed() < timeout {
            let res = RECEIVED_LOGS.lock().unwrap().clone();
            info!("{}/{}: Received data from: {:?}", res.len(), num_cases, res);
            sleep(Duration::from_secs(1)).await;
        }

        assert_eq!(rows.len(), test_data.raw_messages.len());
        assert_eq!(classes.len(), test_data.expected_class.count as usize);
        Ok(())
    }
}
