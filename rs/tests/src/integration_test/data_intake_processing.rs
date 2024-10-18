#[cfg(test)]
mod tests {
    use data_intake::error::DataIntakeError;
    use data_intake::server::initialize_data_intake;
    use data_processing::run::run_data_processing;
    use data_vectorizer::run::run_data_vectorizer;
    use rstest::rstest;
    use shared::connections::get_db_name;
    use shared::connections::greptime::connect::GreptimeConnection;
    use shared::connections::greptime::middleware::query::read_records;
    use shared::connections::qdrant::connect::QdrantConnection;
    use shared::get_env_var;
    use shared::mock::rocket::get_test_client;
    use shared::tracing::setup::setup_tracing;
    use shared::utils::mock::mock_data::{get_test_data, TestCase, TestData};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};
    use std::collections::HashSet;
    use std::sync::{Mutex, Once};
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
    #[case(get_test_data(TestCase::Simple))]
    #[case(get_test_data(TestCase::DataIntakeLimit))]
    #[case(get_test_data(TestCase::DataProcessingLimit))]
    async fn test_data_integration(#[case] test_data: TestData) -> Result<(), DataIntakeError> {
        setup_tracing(true);
        let num_cases = 3;
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
                run_data_vectorizer().await.unwrap();
            });
        });

        let greptime = GreptimeConnection::new().await?;
        let qdrant = QdrantConnection::new().await.unwrap();
        let pod_name = test_data.metadata.pod_name.clone();
        let db_name = get_db_name(&get_env_var("AUTH0_CLIENT_ID_DEV").unwrap());

        let start_time = tokio::time::Instant::now();
        let timeout = Duration::from_secs(30);
        let mut rows = Vec::new();
        let mut classes = Vec::new();

        while start_time.elapsed() < timeout {
            // check greptime
            rows = read_records(greptime.clone(), &db_name, &pod_name)
                .await
                .unwrap();

            // check qdrant
            classes = qdrant.search_key(&db_name, &pod_name).await.unwrap();
            if rows.len() > 0 && classes.len() == test_data.expected_classes.len() {
                // successfully received data
                RECEIVED_DATA.lock().unwrap().insert(pod_name.clone());
                break;
            }
            info!("Waiting for: {}", pod_name);
            sleep(Duration::from_secs(1)).await;
        }

        while RECEIVED_DATA.lock().unwrap().len() < num_cases && start_time.elapsed() < timeout {
            let res = RECEIVED_DATA.lock().unwrap().clone();
            info!("{}/{}: Received data from: {:?}", res.len(), num_cases, res);
            sleep(Duration::from_secs(1)).await;
        }

        assert_eq!(rows.len(), test_data.raw_messages.len());
        assert_eq!(classes.len(), 1);
        Ok(())
    }
}
