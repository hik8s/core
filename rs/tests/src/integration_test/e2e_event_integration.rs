#[cfg(test)]
mod tests {
    use data_intake::error::DataIntakeError;
    use data_intake::server::initialize_data_intake;
    use data_processing::run::run_event_processing;
    use data_vectorizer::run::run_vectorize_event;
    use k8s_openapi::api::core::v1::Event;
    use qdrant_client::qdrant::{ScoredPoint, Value};
    use rstest::rstest;
    use shared::connections::greptime::greptime_connection::{parse_resource_name, GreptimeTable};
    use shared::constant::OPENAI_EMBEDDING_TOKEN_LIMIT;
    use shared::mock::rocket::get_test_client;
    use shared::qdrant_util::{match_any, parse_qdrant_value};
    use shared::setup_tracing;

    use shared::utils::mock::mock_client::post_test_batch;
    use shared::DbName;
    use shared::GreptimeConnection;
    use shared::RateLimiter;
    use shared::{get_env_var, QdrantConnection};
    use std::collections::HashSet;
    use std::path::Path;
    use std::sync::{Arc, Mutex, Once};
    use std::time::{Duration, Instant};
    use tokio::time::sleep;
    use tracing::info;

    use crate::constant::{OWNER_UID, UID};
    use crate::util::{read_yaml_typed, replace_event_uids, TestType};

    static THREAD_EVENT_PROCESSING: Once = Once::new();

    lazy_static::lazy_static! {
        static ref RECEIVED_EVENTS_QDRANT: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        static ref RECEIVED_EVENTS_GREPTIME: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    }

    #[tokio::test]
    #[rstest]
    #[case(("event-filter", "events", DbName::Event, 1,TestType::Update, 1))]
    async fn e2e_event_integration(
        #[case] (subdir, route, dbname, num_points, test_type, num_tables): (
            &str,
            &str,
            DbName,
            usize,
            TestType,
            usize,
        ),
    ) -> Result<(), DataIntakeError> {
        let num_cases = 1;
        setup_tracing(true);
        let qdrant = QdrantConnection::new().await.unwrap();
        let greptime = GreptimeConnection::new().await?;
        let customer_id = get_env_var("CLIENT_ID_LOCAL").unwrap();
        let db = dbname.id(&customer_id);

        THREAD_EVENT_PROCESSING.call_once(|| {
            run_event_processing().unwrap();

            let limiter = Arc::new(RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT));
            run_vectorize_event(limiter).unwrap();
        });

        let server = initialize_data_intake().await.unwrap();

        let client = get_test_client(server).await?;

        let path = Path::new("fixtures").join(subdir);
        let mut events: Vec<shared::types::kubeapidata::KubeApiDataTyped<Event>> =
            read_yaml_typed::<Event>(&path).unwrap();
        // this assumes that the same resource uid is being sent
        let uid_map = replace_event_uids(&mut events);
        let resource_uid = uid_map.get(UID).unwrap().to_string();

        tracing::debug!("test: {subdir} Owner UID map: {uid_map:?}");

        let json: Vec<serde_json::Value> = events.into_iter().map(Into::into).collect();

        let status = post_test_batch(&client, &format!("/{route}"), json).await;
        assert_eq!(status.code, 200);

        let start_time = Instant::now();
        let timeout = Duration::from_secs(15);

        let mut received_greptime = false;
        let mut received_qdrant = false;
        let mut points = Vec::<ScoredPoint>::new();
        let mut tables = Vec::<String>::new();
        while start_time.elapsed() < timeout {
            let search_uid = uid_map
                .get(OWNER_UID)
                .map(ToOwned::to_owned)
                .unwrap_or(resource_uid.clone());

            let filter = match_any("resource_uid", &[search_uid.clone()]);
            points = qdrant
                .query_points(&db, Some(filter), 1000, true)
                .await
                .unwrap();
            if test_type == TestType::Delete {
                points.retain(|point| point.payload.get("deleted") == Some(&Value::from(true)));
            }
            tracing::debug!(
                "subdir: {} len: {}, expected: {}",
                subdir,
                points.len(),
                num_points
            );

            tables = greptime
                .list_tables(&db, Some(&search_uid), None, false)
                .await
                .unwrap();

            let key = match test_type {
                TestType::Delete => format!("{search_uid}___deleted"),
                TestType::Update => search_uid.to_owned(),
            };

            if !received_greptime && tables.iter().any(|table| table.contains(&key)) {
                RECEIVED_EVENTS_GREPTIME
                    .lock()
                    .unwrap()
                    .insert(subdir.to_string());
                received_greptime = true;
            }

            if !received_qdrant && points.len() == num_points {
                RECEIVED_EVENTS_QDRANT
                    .lock()
                    .unwrap()
                    .insert(subdir.to_string());
                received_qdrant = true;
            }

            if received_qdrant && received_greptime {
                break;
            }

            sleep(Duration::from_secs(5)).await;
        }
        for point in points.clone() {
            if point.payload.get("data_type") == Some(&Value::from("status")) {
                let (yaml, _json) = parse_qdrant_value(point.payload.get("data").unwrap());
                tracing::debug!("status: {:#?}", yaml);
            }
        }

        let mut res_qdrant_len = 0;
        let mut res_greptime_len = 0;

        while (res_qdrant_len < num_cases || res_greptime_len < num_cases)
            && start_time.elapsed() < timeout
        {
            info!(
                "qdrant({}/{}) | greptime ({}/{})",
                res_qdrant_len, num_cases, res_greptime_len, num_cases
            );
            sleep(Duration::from_secs(1)).await;
            res_qdrant_len = RECEIVED_EVENTS_QDRANT.lock().unwrap().len();
            res_greptime_len = RECEIVED_EVENTS_GREPTIME.lock().unwrap().len();
        }

        assert_eq!(
            points.len(),
            num_points,
            "Expected {num_points} points in Qdrant for '{subdir}', but found {}",
            points.len()
        );
        assert_eq!(
            tables.len(),
            num_tables,
            "Expected {num_tables} tables in GreptimeDB for '{subdir}', but found {}",
            tables.len()
        );
        if test_type == TestType::Delete {
            // Log for debugging
            tracing::debug!(
                "Verified all {} tables are properly marked as deleted: {:?}",
                tables.len(),
                tables
            );

            // Parse all table names and ensure they're all marked as deleted
            let parsed_tables: Vec<GreptimeTable> = tables
                .iter()
                .filter_map(|table| parse_resource_name(table))
                .collect();

            assert!(
                !parsed_tables.is_empty(),
                "Should have at least one table to check"
            );

            // Verify each table is marked as deleted
            let all_deleted = parsed_tables.iter().all(|table| table.is_deleted);
            assert!(
                all_deleted,
                "All tables should be marked as deleted, found non-deleted tables: {:?}",
                parsed_tables
                    .iter()
                    .filter(|t| !t.is_deleted)
                    .collect::<Vec<_>>()
            );
        }
        // random comment for git
        assert!(received_qdrant);
        assert!(received_greptime);
        Ok(())
    }
}
