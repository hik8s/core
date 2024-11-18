use crate::error::DataIntakeError;

use rocket::post;
use rocket::serde::json::Json;
use shared::fluvio::{FluvioConnection, TopicName};
use shared::log_error;
use shared::router::auth::guard::AuthenticatedUser;

#[post("/resource", format = "json", data = "<resource>")]
pub async fn resource_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    resource: Json<serde_json::Value>,
) -> Result<String, DataIntakeError> {
    let producer = fluvio.get_producer(TopicName::Resource);
    producer
        .send(user.customer_id.clone(), resource.into_inner().to_string())
        .await
        .map_err(|e| log_error!(e))
        .ok();
    producer.flush().await.map_err(|e| log_error!(e)).ok();

    Ok("Success".to_string())
}

#[post("/resources", format = "json", data = "<resources>")]
pub async fn resources_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    resources: Json<Vec<serde_json::Value>>,
) -> Result<String, DataIntakeError> {
    let producer = fluvio.get_producer(TopicName::Resource);

    for resource in resources.into_inner() {
        producer
            .send(user.customer_id.clone(), resource.to_string())
            .await
            .map_err(|e| log_error!(e))
            .ok();
    }

    producer.flush().await.map_err(|e| log_error!(e)).ok();

    Ok("Success".to_string())
}
