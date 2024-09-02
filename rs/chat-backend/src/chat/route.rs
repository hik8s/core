use rocket::post;

use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::Json;
use shared::connections::prompt_engine::connect::PromptEngineConnection;
use shared::openai::chat_complete::{process_user_message, RequestOptions};
use tokio;
use tokio::sync::mpsc;

use super::header::LastEventId;

#[post("/chat/completions", format = "json", data = "<payload>")]
pub fn chat_completion(
    id: LastEventId,
    prompt_engine: PromptEngineConnection,
    payload: Json<RequestOptions>,
) -> EventStream![] {
    let id = id.0;
    let (tx, mut rx) = mpsc::unbounded_channel();

    // producer: openai api (tx)
    tokio::spawn(async move {
        match process_user_message(prompt_engine, payload.into_inner(), move |chat_message| {
            tx.send(chat_message).ok();
        })
        .await
        {
            Ok(()) => {
                tracing::info!("Chat process done");
            }
            Err(err) => {
                tracing::error!("Chat process error: {:?}", err);
            }
        }
    });

    // consumer: client (rx)
    EventStream! {
        while let Some(chat_message) = rx.recv().await {
            tracing::debug!("Yield: {:?}", chat_message);
            let event = Event::json(&chat_message.delta).id(id.to_string());
            yield event;
        }
    }
}
