use crate::log::error::LogIntakeError;

use rocket::post;
use rocket::serde::json::Json;
use shared::fluvio::{FluvioConnection, TopicName};
use shared::log_error;
use shared::router::auth::guard::AuthenticatedUser;

#[post("/customresource", format = "json", data = "<event>")]
pub async fn customresource_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    event: Json<serde_json::Value>,
) -> Result<String, LogIntakeError> {
    let producer = fluvio.get_producer(TopicName::CustomResource);
    producer
        .send(user.customer_id.clone(), event.into_inner().to_string())
        .await
        .map_err(|e| log_error!(e))
        .ok();
    producer.flush().await.map_err(|e| log_error!(e)).ok();

    Ok("Success".to_string())
}
