// #[cfg(test)]
// mod tests {
//     use data_intake::error::DataIntakeError;
//     use data_intake::server::initialize_data_intake;
//     use rstest::rstest;
//     use shared::mock::rocket::get_test_client;
//     use shared::tracing::setup::setup_tracing;

//     use crate::util::process_resource_files;

//     #[tokio::test]
//     #[rstest]
//     #[case("customresources")]
//     #[case("events")]
//     #[case("resources")]
//     async fn test_resource_processing(#[case] route: &str) -> Result<(), DataIntakeError> {
//         setup_tracing(true);

//         let server = initialize_data_intake().await.unwrap();

//         let client = get_test_client(server).await?;

//         process_resource_files(client, route).await?;
//         Ok(())
//     }
// }
