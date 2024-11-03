use shared::connections::{
    openai::{
        chat_complete::request_completion,
        messages::{create_assistant_message, create_system_message, create_user_message},
    },
    prompt_engine::connect::PromptEngineConnection,
    OpenAIConnection,
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
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub async fn process_user_message(
    prompt_engine: &PromptEngineConnection,
    payload: RequestOptions,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), anyhow::Error> {
    let mut last_user_content = String::new();
    let mut messages = payload
        .messages
        .into_iter()
        .map(|message| match message.role.as_str() {
            "system" => create_system_message(),
            "user" => {
                last_user_content = message.content.clone();
                create_user_message(&message.content)
            }
            "assistant" => create_assistant_message(&message.content, None),
            _ => panic!("Unknown role"),
        })
        .collect();

    let openai = OpenAIConnection::new();

    request_completion(
        prompt_engine,
        &openai,
        &mut messages,
        last_user_content,
        payload.client_id.clone(),
        &payload.model,
        tx,
    )
    .await?;
    Ok(())
}
