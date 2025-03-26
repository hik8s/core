use async_openai::types::ChatCompletionRequestMessage;
use rocket::post;

use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::Json;

use shared::constant::OPENAI_CHAT_MODEL_MINI;
use shared::openai_util::{create_assistant_message, create_system_message, create_user_message};
use shared::{GreptimeConnection, QdrantConnection};
use tokio;
use tokio::sync::mpsc;
use tracing::error;

use crate::handle::handle_chat_completion::process_user_message;

use super::header::LastEventId;

#[post("/chat/completions", format = "json", data = "<payload>")]
pub fn chat_completion(
    id: LastEventId,
    greptime: GreptimeConnection,
    qdrant: QdrantConnection,
    payload: Json<RequestOptions>,
) -> EventStream![] {
    let id = id.0;
    let (tx, mut rx) = mpsc::unbounded_channel();

    // producer: openai api (tx)
    let request_options = payload.into_inner();
    let mut messages: Vec<ChatCompletionRequestMessage> = request_options.clone().into();

    tokio::spawn(async move {
        process_user_message(&greptime, &qdrant, &mut messages, &tx, request_options)
            .await
            .map_err(|e| error!("{e}"))
            .ok();
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RequestOptions {
    pub messages: Vec<Message>,
    pub model: String,
    pub client_id: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub iteration_depth: Option<usize>,
}
impl RequestOptions {
    pub fn test(input: &str, client_id: &str) -> Self {
        RequestOptions {
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "not used".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: input.to_string(),
                },
            ],
            model: OPENAI_CHAT_MODEL_MINI.to_string(),
            client_id: client_id.to_owned(),
            temperature: None,
            top_p: None,
            iteration_depth: Some(1),
        }
    }
}
impl From<RequestOptions> for Vec<ChatCompletionRequestMessage> {
    fn from(val: RequestOptions) -> Self {
        val.messages
            .into_iter()
            .map(|message| match message.role.as_str() {
                "system" => create_system_message(),
                "user" => create_user_message(&message.content),
                "assistant" => create_assistant_message(&message.content, None),
                _ => panic!("Unknown role in message: {message:?}"),
            })
            .collect()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}
