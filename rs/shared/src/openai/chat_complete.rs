use std::future::Future;
use std::pin::Pin;

use crate::connections::openai::tools::{collect_tool_call_chunks, Tool};
use crate::connections::prompt_engine::connect::PromptEngineConnection;
use crate::connections::OpenAIConnection;
use crate::log_error;
use crate::openai::chat_request_args::create_tool_message;
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestMessage, FinishReason,
};

use rocket::FromForm;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::info;

use super::chat_request_args::{
    create_assistant_message, create_system_message, create_user_message,
};

#[derive(FromForm, Serialize, Debug)]
// serialization is used to preserve spaces
pub struct ChatMessage {
    pub id: String,
    pub delta: String,
    text: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RequestOptions {
    messages: Vec<Message>,
    model: String,
    client_id: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct Message {
    role: String,
    content: String,
}

pub async fn process_user_message(
    prompt_engine: &PromptEngineConnection,
    payload: &RequestOptions,
    tx: &mpsc::UnboundedSender<String>,
    tool_calls: Vec<ChatCompletionMessageToolCall>,
) -> Result<(), anyhow::Error> {
    info!("Messages: {:#?}", payload.messages);

    let mut last_user_content = String::new();
    let mut messages: Vec<ChatCompletionRequestMessage> = payload
        .clone()
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
    info!("Last user message content: {:?}", last_user_content);

    for tool_call in tool_calls {
        let tool = Tool::try_from(&tool_call.function.name).unwrap();
        let tool_output = tool
            .request(prompt_engine, &last_user_content, &payload.client_id)
            .await?;
        let tool = create_tool_message(&tool_output, &tool_call.id);
        messages.push(tool);
    }
    info!("Messages: {:#?}", messages);

    let openai = OpenAIConnection::new();

    let request = openai
        .complete_request(messages, &payload.model)
        .map_err(|e| log_error!(e))?;

    let stream = openai
        .create_completion_stream(request)
        .await
        .map_err(|e| log_error!(e))?;

    // info!("{stream:#?}");
    let (finish_reason, tool_call_chunks) = openai
        .process_completion_stream(tx, stream)
        .await
        .map_err(|e| log_error!(e))?;
    let tool_calls = collect_tool_call_chunks(tool_call_chunks);

    info!("Tool calls: {:#?}", tool_calls);
    if finish_reason == Some(FinishReason::Stop) {
        return Ok(());
    }
    process_user_message_recursive(prompt_engine, payload, tx, tool_calls)
        .await
        .unwrap();
    Ok(())
}

fn process_user_message_recursive<'a>(
    prompt_engine: &'a PromptEngineConnection,
    payload: &'a RequestOptions,
    tx: &'a mpsc::UnboundedSender<String>,
    tool_calls: Vec<ChatCompletionMessageToolCall>,
) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + 'a>> {
    Box::pin(async move {
        // Your recursive logic here
        // For example, you can call process_user_message again
        process_user_message(prompt_engine, payload, tx, tool_calls).await
    })
}
