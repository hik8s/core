#[cfg(test)]
mod tests {
    use rocket::http::ContentType;
    use rstest::rstest;
    use shared::{
        connections::prompt_engine::connect::AugmentationRequest, get_env_var,
        mock::rocket::get_test_client, tracing::setup::setup_tracing,
    };

    #[tokio::test]
    #[rstest]
    #[case(ClusterTestScenario::PodKillOutOffMemory)]
    async fn test_prompt(#[case] test_scenario: ClusterTestScenario) {
        setup_tracing(false);
        // Prepare the test scenario data
        let scenario_data = get_test_scenario_data(test_scenario);

        let server = initialize_prompt_engine().await.unwrap();
        let client = get_test_client(server).await.unwrap();

        let client_id = get_env_var("AUTH0_CLIENT_ID_DEV").unwrap();
        let body: String = AugmentationRequest::new("Test message", &client_id)
            .try_into()
            .unwrap();

        let response = client
            .post("/prompt")
            .header(ContentType::JSON)
            .body(body)
            .dispatch()
            .await;

        // Add assertions to check the response
        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body = response.into_string().await.unwrap();
        assert!(body.len() > 500);
    }
}
