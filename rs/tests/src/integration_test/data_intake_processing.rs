#[cfg(test)]
mod tests {
    use data_intake::error::DataIntakeError;
    use data_intake::server::initialize_data_intake;
    use data_processing::run::run_data_processing;
    use shared::mock::rocket::get_test_client;
    use shared::tracing::setup::setup_tracing;
    use shared::utils::mock::mock_data::{get_test_data, TestCase};
    use shared::utils::mock::{mock_client::post_test_stream, mock_stream::get_multipart_stream};
    use test_case::test_case;

    #[tokio::test]
    #[test_case(TestCase::Simple)]
    #[test_case(TestCase::DataIntakeLimit)]
    async fn test_data_intake_processing(case: TestCase) -> Result<(), DataIntakeError> {
        setup_tracing();

        run_data_processing().await.unwrap();

        // rocket client
        let server = initialize_data_intake().await.unwrap();
        let client = get_test_client(server).await?;

        // test data
        let test_data = get_test_data(case);
        let test_stream = get_multipart_stream(test_data);

        // test route
        let status = post_test_stream(&client, "/logs", test_stream).await;
        assert_eq!(status.code, 200);
        Ok(())
    }
}
