#[cfg(test)]
mod tests {
    use data_intake::log::route::log_intake;
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
    async fn test_data_intake_processing(case: TestCase) -> Result<(), TestClientError> {
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
