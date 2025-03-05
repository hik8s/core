use crate::error::DataIntakeError;

use rocket::post;
use rocket::serde::json::Json;
use shared::fluvio::TopicName;
use shared::router::auth::guard::AuthenticatedUser;
use shared::{log_error, FluvioConnection};

#[post("/event", format = "json", data = "<event>")]
pub async fn event_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    event: Json<serde_json::Value>,
) -> Result<String, DataIntakeError> {
    let producer = fluvio.get_producer(TopicName::Event);
    producer
        .send(user.customer_id.clone(), event.into_inner().to_string())
        .await
        .map_err(|e| log_error!(e))
        .ok();
    producer.flush().await.map_err(|e| log_error!(e)).ok();

    Ok("Success".to_string())
}

#[post("/events", format = "json", data = "<events>")]
pub async fn events_intake(
    user: AuthenticatedUser,
    fluvio: FluvioConnection,
    events: Json<Vec<serde_json::Value>>,
) -> Result<String, DataIntakeError> {
    let producer = fluvio.get_producer(TopicName::Event);
    for event in events.into_inner() {
        producer
            .send(user.customer_id.clone(), event.to_string())
            .await
            .map_err(|e| log_error!(e))
            .ok();
    }
    producer.flush().await.map_err(|e| log_error!(e)).ok();

    Ok("Success".to_string())
}
