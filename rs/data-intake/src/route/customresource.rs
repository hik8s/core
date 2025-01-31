use crate::error::DataIntakeError;

use rocket::post;
use rocket::serde::json::Json;
use shared::fluvio::{FluvioConnection, TopicName};
use shared::log_error;
use shared::router::auth::guard::AuthenticatedUser;
use shared::types::kubeapidata::KubeApiData;
use shared::utils::get_as_string;

#[post("/customresource", format = "json", data = "<customresource>")]
pub async fn customresource_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    customresource: Json<serde_json::Value>,
) -> Result<String, DataIntakeError> {
    let data: KubeApiData = customresource
        .into_inner()
        .try_into()
        .map_err(|e| DataIntakeError::DeserializationError(log_error!(e)))?;

    let kind = get_as_string(&data.json, "kind")
        .map_err(|e| log_error!(e))?
        .to_lowercase();
    if kind == "partition"
        || kind == "kustomization"
        || kind == "gitrepository"
        || kind == "helmchart"
        || kind == "ciliumendpoint"
        || kind == "ciliumidentity"
        || kind == "policyreport"
        || kind == "ephemeralreport"
    {
        return Ok(format!("Skip {}", kind));
    }

    let producer = fluvio.get_producer(TopicName::CustomResource);
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

#[post("/customresources", format = "json", data = "<customresources>")]
pub async fn customresources_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    customresources: Json<Vec<serde_json::Value>>,
) -> Result<String, DataIntakeError> {
    let producer = fluvio.get_producer(TopicName::CustomResource);

    for cr in customresources.into_inner() {
        let data: KubeApiData = cr
            .try_into()
            .map_err(|e| DataIntakeError::DeserializationError(log_error!(e)))?;

        let kind = get_as_string(&data.json, "kind")
            .map_err(|e| log_error!(e))?
            .to_lowercase();
        if kind == "partition"
            || kind == "kustomization"
            || kind == "ciliumendpoint"
            || kind == "ciliumidentity"
        {
            continue;
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
