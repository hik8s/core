use crate::error::DataIntakeError;

use rocket::post;
use rocket::serde::json::Json;
use shared::fluvio::{FluvioConnection, TopicName};
use shared::log_error;
use shared::router::auth::guard::AuthenticatedUser;

#[post("/customresource", format = "json", data = "<customresource>")]
pub async fn customresource_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    customresource: Json<serde_json::Value>,
) -> Result<String, DataIntakeError> {
    // let resource_json = customresource.clone().into_inner();
    // save_resource(&resource_json, "customresource").unwrap();
    let producer = fluvio.get_producer(TopicName::CustomResource);
    producer
        .send(
            user.customer_id.clone(),
            customresource.into_inner().to_string(),
        )
        .await
        .map_err(|e| log_error!(e))
        .ok();
    producer.flush().await.map_err(|e| log_error!(e)).ok();

    Ok("Success".to_string())
}

#[post("/customresources", format = "json", data = "<customresources>")]
pub async fn customresources_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    customresources: Json<Vec<serde_json::Value>>,
) -> Result<String, DataIntakeError> {
    let producer = fluvio.get_producer(TopicName::CustomResource);
    for cr in customresources.into_inner() {
        producer
            .send(user.customer_id.clone(), cr.to_string())
            .await
            .map_err(|e| log_error!(e))
            .ok();
    }
    producer.flush().await.map_err(|e| log_error!(e)).ok();

    Ok("Success".to_string())
}
