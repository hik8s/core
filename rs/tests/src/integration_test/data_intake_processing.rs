#[cfg(test)]
mod tests {
    use data_intake::error::DataIntakeError;
    use data_intake::server::initialize_data_intake;
    use data_processing::run::run_data_processing;
    use data_vectorizer::run::run_data_vectorizer;
    use rstest::rstest;
    use shared::connections::db_name::get_db_name;
    use shared::connections::greptime::connect::GreptimeConnection;
    use shared::connections::greptime::middleware::query::read_records;
    use shared::connections::qdrant::connect::QdrantConnection;
    use shared::get_env_var;
    use shared::mock::rocket::get_test_client;
    use shared::tracing::setup::setup_tracing;
    use shared::utils::mock::mock_data::{get_test_data, TestCase};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    #[rstest]
    #[case(TestCase::Simple)]
    // #[case(TestCase::DataIntakeLimit)]
    async fn test_data_integration(#[case] case: TestCase) -> Result<(), DataIntakeError> {
        setup_tracing(true);

        // test data
        let test_data = get_test_data(case);
        let test_stream = get_multipart_stream(&test_data);
        let customer_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
        let db_name = get_db_name(&customer_id);
        let table_name = test_data.metadata.pod_name;

        // data processing
        run_data_processing().await.unwrap();

        // data vectorizer
        tokio::spawn(async move {
            run_data_vectorizer().await.unwrap();
        });

        // data intake
        let server = initialize_data_intake().await.unwrap();
        let client = get_test_client(server).await?;

        // send stream
        let status = post_test_stream(&client, "/logs", test_stream).await;
        assert_eq!(status.code, 200);
        sleep(Duration::from_secs(3)).await;

        // check greptime
        let greptime = GreptimeConnection::new().await?;
        let rows = read_records(greptime, &db_name, &table_name).await.unwrap();
        assert_eq!(rows.len(), test_data.raw_messages.len());

        // check qdrant
        let qdrant = QdrantConnection::new().await.unwrap();
        let classes = qdrant.search_key(&db_name, &table_name).await.unwrap();
        assert_eq!(classes.len(), 1);
        Ok(())
    }
}
