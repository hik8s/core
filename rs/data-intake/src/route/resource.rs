use crate::error::DataIntakeError;

use rocket::post;
use rocket::serde::json::Json;
use shared::fluvio::{FluvioConnection, TopicName};
use shared::log_error;
use shared::router::auth::guard::AuthenticatedUser;
use shared::types::kubeapidata::KubeApiData;
use shared::utils::get_as_ref;

#[post("/resource", format = "json", data = "<resource>")]
pub async fn resource_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    resource: Json<serde_json::Value>,
) -> Result<String, DataIntakeError> {
    let data: KubeApiData = resource
        .into_inner()
        .try_into()
        .map_err(|e| DataIntakeError::DeserializationError(log_error!(e)))?;
    let producer = fluvio.get_producer(TopicName::Resource);

    let metadata = get_as_ref(&data.json, "metadata").map_err(|e| log_error!(e))?;
    if let Some(owner_refs) = metadata.get("ownerReferences") {
        if let Some(refs) = owner_refs.as_array() {
            if refs.len() == 1 {
                if let Some(owner) = refs.first() {
                    if owner.get("kind").and_then(|k| k.as_str()) == Some("Job") {
                        return Ok("Skipping Job pod".to_string());
                    }
                }
            }
        }
    }
    let data_ser: Vec<u8> = data
        .try_into()
        .map_err(|e| DataIntakeError::SerializationError(log_error!(e)))?;
    producer
        .send(user.customer_id.clone(), data_ser)
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
        let data: KubeApiData = resource
            .try_into()
            .map_err(|e| DataIntakeError::DeserializationError(log_error!(e)))?;

        let metadata = get_as_ref(&data.json, "metadata").map_err(|e| log_error!(e))?;
        if let Some(owner_refs) = metadata.get("ownerReferences") {
            if let Some(refs) = owner_refs.as_array() {
                if refs.len() == 1 {
                    if let Some(owner) = refs.first() {
                        if owner.get("kind").and_then(|k| k.as_str()) == Some("Job") {
                            continue;
                        }
                    }
                }
            }
        }

        let data_ser: Vec<u8> = data
            .try_into()
            .map_err(|e| DataIntakeError::SerializationError(log_error!(e)))?;
        producer
            .send(user.customer_id.clone(), data_ser)
            .await
            .map_err(|e| log_error!(e))
            .ok();
    }

    producer.flush().await.map_err(|e| log_error!(e)).ok();
    Ok("Success".to_string())
}
