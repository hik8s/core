#[cfg(test)]
mod tests {
    use data_intake::error::DataIntakeError;
    use data_intake::server::initialize_data_intake;
    use data_processing::run::run_data_processing;
    use data_vectorizer::vectorize_class::vectorize_class;
    use rstest::rstest;
    use shared::connections::dbname::DbName;
    use shared::connections::greptime::connect::GreptimeConnection;
    use shared::connections::greptime::middleware::query::read_records;
    use shared::connections::qdrant::connect::QdrantConnection;
    use shared::constant::OPENAI_EMBEDDING_TOKEN_LIMIT;
    use shared::get_env_var;
    use shared::mock::rocket::get_test_client;
    use shared::tracing::setup::setup_tracing;
    use shared::utils::mock::mock_data::{get_test_data, TestCase};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};
    use shared::utils::ratelimit::RateLimiter;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex, Once};
    use std::time::Duration;
    use tracing::info;

    use tokio::time::sleep;

    static THREAD_VECTORIZER: Once = Once::new();
    static THREAD_PROCESSING: Once = Once::new();

    lazy_static::lazy_static! {
        static ref RECEIVED_DATA: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    }

    #[tokio::test]
    #[rstest]
    #[case(TestCase::Simple)]
    #[case(TestCase::DataIntakeLimit)]
    #[case(TestCase::DataProcessingLimit)]
    // #[case(TestCase::OpenAiRateLimit)]
    async fn test_data_integration(#[case] case: TestCase) -> Result<(), DataIntakeError> {
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
        THREAD_PROCESSING.call_once(|| {
            tokio::spawn(async move {
                run_data_processing().await.unwrap();
            });
        });
        // data vectorizer
        THREAD_VECTORIZER.call_once(|| {
            tokio::spawn(async move {
                let limiter = Arc::new(RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT));
                vectorize_class(limiter).await.unwrap();
            });
        });

        let greptime = GreptimeConnection::new().await?;
        let qdrant = QdrantConnection::new().await.unwrap();
        let pod_name = test_data.metadata.pod_name.clone();
        let customer_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
        let db = DbName::Log;
        qdrant.create_collection(&db, &customer_id).await.unwrap();

        let start_time = tokio::time::Instant::now();
        let timeout = Duration::from_secs(30);
        let mut rows = Vec::new();
        let mut classes = Vec::new();

        while start_time.elapsed() < timeout {
            // check greptime
            rows = read_records(greptime.clone(), &db, &customer_id, &pod_name)
                .await
                .unwrap();

            // check qdrant
            classes = qdrant
                .search_key(&db, &customer_id, &pod_name, 1000)
                .await
                .unwrap();
            info!(
                "Classes: {}/{} | Pod: {}",
                classes.len(),
                test_data.expected_class.count,
                pod_name
            );
            if rows.len() > 0 && classes.len() == test_data.expected_class.count as usize {
                // successfully received data
                RECEIVED_DATA.lock().unwrap().insert(pod_name.clone());
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }

        while RECEIVED_DATA.lock().unwrap().len() < num_cases && start_time.elapsed() < timeout {
            let res = RECEIVED_DATA.lock().unwrap().clone();
            info!("{}/{}: Received data from: {:?}", res.len(), num_cases, res);
            sleep(Duration::from_secs(1)).await;
        }

        assert_eq!(rows.len(), test_data.raw_messages.len());
        assert_eq!(classes.len(), test_data.expected_class.count as usize);
        Ok(())
    }
}
