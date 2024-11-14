use async_openai::types::ChatCompletionRequestMessage;
use rocket::post;

use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::Json;

use shared::connections::qdrant::connect::QdrantConnection;
use tokio;
use tokio::sync::mpsc;
use tracing::error;

use super::header::LastEventId;
use super::process::process_user_message;
use super::process::RequestOptions;

#[post("/chat/completions", format = "json", data = "<payload>")]
pub fn chat_completion(
    id: LastEventId,
    qdrant: QdrantConnection,
    payload: Json<RequestOptions>,
) -> EventStream![] {
    let id = id.0;
    let (tx, mut rx) = mpsc::unbounded_channel();

    // producer: openai api (tx)
    let request_options = payload.into_inner();
    let mut messages: Vec<ChatCompletionRequestMessage> = request_options.clone().into();

    tokio::spawn(async move {
        process_user_message(&qdrant, &mut messages, &tx, request_options)
            .await
            .map_err(|e| error!("{e}"))
            .ok()
    });

    // consumer: client (rx)
    EventStream! {
        while let Some(message_delta) = rx.recv().await {
            tracing::debug!("Yield: {:?}", message_delta);
            let event = Event::json(&message_delta).id(id.to_string());
            yield event;
        }
    }
}
