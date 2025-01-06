#[cfg(test)]
mod tests {
    use data_intake::error::DataIntakeError;
    use data_intake::server::initialize_data_intake;
    use data_processing::run::{
        run_customresource_processing, run_data_processing, run_resource_processing,
    };
    use data_vectorizer::run::{run_vectorize_customresource, run_vectorize_resource};
    use data_vectorizer::vectorize_class;
    use qdrant_client::qdrant::{ScoredPoint, Value};
    use rstest::rstest;
    use shared::connections::dbname::DbName;
    use shared::connections::greptime::connect::GreptimeConnection;
    use shared::connections::greptime::middleware::query::read_records;
    use shared::connections::qdrant::connect::{match_any, string_filter, QdrantConnection};
    use shared::constant::OPENAI_EMBEDDING_TOKEN_LIMIT;
    use shared::get_env_var;
    use shared::mock::rocket::get_test_client;
    use shared::tracing::setup::setup_tracing;
    use shared::types::class::vectorized::{from_scored_point, VectorizedClass};
    use shared::utils::mock::mock_client::post_test_batch;
    use shared::utils::mock::mock_data::{get_test_data, TestCase};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};
    use shared::utils::ratelimit::RateLimiter;
    use std::collections::HashSet;
    use std::path::Path;
    use std::sync::{Arc, Mutex, Once};
    use std::time::{Duration, Instant};
    use tracing::info;
    use uuid7::uuid4;

    use tokio::time::sleep;

    use crate::util::read_yaml_files;

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
    async fn test_log_integration(#[case] case: TestCase) -> Result<(), DataIntakeError> {
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
            run_data_processing().unwrap();

            // data vectorizer
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
        let mut classes: Vec<VectorizedClass> = Vec::new();

        while start_time.elapsed() < timeout {
            // check greptime
            rows = read_records(greptime.clone(), &db, &customer_id, &pod_name)
                .await
                .unwrap();

            // check qdrant
            let filter = string_filter("key", &pod_name);
            let points = qdrant
                .query_points(&db, &customer_id, filter, 1000)
                .await
                .unwrap();
            classes = from_scored_point(points).unwrap();
            info!(
                "Classes: {}/{} | Pod: {}",
                classes.len(),
                test_data.expected_class.count,
                pod_name
            );
            if !rows.is_empty() && classes.len() == test_data.expected_class.count as usize {
                // successfully received data
                RECEIVED_LOGS.lock().unwrap().insert(pod_name.clone());
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

    fn replace_resource_uids(resources: &mut [serde_json::Value]) -> String {
        let resource_uid = uuid4().to_string();

        for resource in resources.iter_mut() {
            if let Some(json) = resource.get_mut("json") {
                if let Some(metadata) = json.as_object_mut().and_then(|obj| obj.get_mut("metadata"))
                {
                    if let Some(metadata_obj) = metadata.as_object_mut() {
                        metadata_obj.insert(
                            "uid".to_string(),
                            serde_json::Value::String(resource_uid.clone()),
                        );
                    }
                }
            }
        }

        resource_uid
    }

    static THREAD_RESOURCE_PROCESSING: Once = Once::new();

    lazy_static::lazy_static! {
        static ref RECEIVED_RESOURCES: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    }

    #[tokio::test]
    #[rstest]
    #[case(("pod-deletion", "resources", DbName::Resource, 3))]
    #[case(("certificate-deletion", "customresources", DbName::CustomResource, 3))]
    async fn integration_pod_deletion(
        #[case] (subdir, route, db, num_points): (&str, &str, DbName, usize),
    ) -> Result<(), DataIntakeError> {
        let num_cases = 2;
        setup_tracing(true);
        let qdrant = QdrantConnection::new().await.unwrap();
        let customer_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();

        THREAD_RESOURCE_PROCESSING.call_once(|| {
            run_resource_processing().unwrap();
            run_customresource_processing().unwrap();

            let limiter = Arc::new(RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT));
            run_vectorize_resource(limiter.clone()).unwrap();
            run_vectorize_customresource(limiter).unwrap();
        });

        let server = initialize_data_intake().await.unwrap();

        let client = get_test_client(server).await?;

        let path = Path::new("fixtures").join(subdir);
        let mut json = read_yaml_files(&path).unwrap();

        // this assumes that the same resource uid is being sent
        let resource_uid = replace_resource_uids(&mut json);

        let status = post_test_batch(&client, &format!("/{route}"), json).await;
        assert_eq!(status.code, 200);

        let start_time = Instant::now();
        let timeout = Duration::from_secs(30);
        let mut points = Vec::<ScoredPoint>::new();

        while start_time.elapsed() < timeout {
            let filter = match_any("resource_uid", &[resource_uid.clone()]);
            points = qdrant
                .query_points(&db, &customer_id, filter, 1000)
                .await
                .unwrap();

            info!("Points with deleted=true: {}/{}", points.len(), num_points);

            if points.len() == num_points {
                let all_deleted = points
                    .iter()
                    .all(|point| point.payload.get("deleted") == Some(&Value::from(true)));
                if all_deleted {
                    RECEIVED_RESOURCES
                        .lock()
                        .unwrap()
                        .insert(resource_uid.clone());
                    break;
                }
            }
            sleep(Duration::from_secs(1)).await;
        }

        while RECEIVED_RESOURCES.lock().unwrap().len() < 1 && start_time.elapsed() < timeout {
            let res = RECEIVED_RESOURCES.lock().unwrap().clone();
            info!("{}/{}: Received data from: {:?}", res.len(), num_cases, res);
            sleep(Duration::from_secs(1)).await;
        }
        assert_eq!(points.len(), num_points);
        Ok(())
    }
}
