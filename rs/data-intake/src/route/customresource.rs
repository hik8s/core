use crate::error::DataIntakeError;

use rocket::post;
use rocket::serde::json::Json;
use shared::fluvio::{FluvioConnection, TopicName};
use shared::log_error;
use shared::router::auth::guard::AuthenticatedUser;
use shared::utils::get_as_string;

#[post("/customresource", format = "json", data = "<customresource>")]
pub async fn customresource_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    customresource: Json<serde_json::Value>,
) -> Result<String, DataIntakeError> {
    let cr = customresource.into_inner();
    let kind = get_as_string(&cr, "kind").map_err(|e| log_error!(e));

    if let Ok(kind) = kind {
        if kind.to_lowercase() == "partition" {
            tracing::info!("Partition resource detected, skipping");
            return Ok("Success".to_string());
        }
    }

    let producer = fluvio.get_producer(TopicName::CustomResource);
    producer
        .send(user.customer_id.clone(), cr.to_string())
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
        let kind = get_as_string(&cr, "kind").map_err(|e| log_error!(e));

        if let Ok(kind) = kind {
            if kind.to_lowercase() == "partition" {
                tracing::info!("Partition resource detected, skipping");
                continue;
            }
        }
        producer
            .send(user.customer_id.clone(), cr.to_string())
            .await
            .map_err(|e| log_error!(e))
            .ok();
    }
    producer.flush().await.map_err(|e| log_error!(e)).ok();

    Ok("Success".to_string())
}
