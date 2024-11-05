use async_openai::types::ChatCompletionRequestMessage;
use shared::{
    connections::{
        openai::{
            chat_complete::request_completion,
            messages::{create_assistant_message, create_system_message, create_user_message},
        },
        prompt_engine::connect::PromptEngineConnection,
        OpenAIConnection,
    },
    constant::OPENAI_CHAT_MODEL_MINI,
};
use tokio::sync::mpsc;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RequestOptions {
    pub messages: Vec<Message>,
    pub model: String,
    pub client_id: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}
impl RequestOptions {
    pub fn new(input: &str, client_id: &str) -> Self {
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
        }
    }
}
impl Into<Vec<ChatCompletionRequestMessage>> for RequestOptions {
    fn into(self) -> Vec<ChatCompletionRequestMessage> {
        self.messages
            .into_iter()
            .map(|message| match message.role.as_str() {
                "system" => create_system_message(),
                "user" => create_user_message(&message.content),
                "assistant" => create_assistant_message(&message.content, None),
                _ => panic!("Unknown role"),
            })
            .collect()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub async fn process_user_message(
    prompt_engine: &PromptEngineConnection,
    mut messages: &mut Vec<ChatCompletionRequestMessage>,
    tx: &mpsc::UnboundedSender<String>,
    payload: RequestOptions,
) -> Result<(), anyhow::Error> {
    let openai = OpenAIConnection::new();

    request_completion(
        prompt_engine,
        &openai,
        &mut messages,
        payload.client_id.clone(),
        &payload.model,
        tx,
    )
    .await?;
    Ok(())
}
