use std::future::Future;
use std::pin::Pin;

use crate::connections::openai::tools::{collect_tool_call_chunks, Tool};
use crate::connections::prompt_engine::connect::PromptEngineConnection;
use crate::connections::OpenAIConnection;
use crate::log_error;
use crate::openai::chat_request_args::create_tool_message;
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk,
    ChatCompletionRequestMessage, FinishReason,
};

use futures_util::StreamExt;
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
    tx: &mpsc::UnboundedSender<ChatMessage>,
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

    let mut stream = openai
        .chat_stream(request)
        .await
        .map_err(|e| log_error!(e))?;

    // info!("{stream:#?}");
    let mut tool_call_chunks = Vec::<ChatCompletionMessageToolCallChunk>::new();
    let mut finish_reason = Option::<FinishReason>::None;
    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                response.choices.iter().for_each(|chat_choice| {
                    if let Some(ref tool_call_chunk) = chat_choice.delta.tool_calls {
                        tool_call_chunks.extend(tool_call_chunk.clone());
                    }
                    if let Some(ref content) = chat_choice.delta.content {
                        let chat_message = ChatMessage {
                            id: chat_choice.index.to_string(),
                            text: content.clone(),
                            delta: chat_choice
                                .delta
                                .content
                                .clone()
                                .unwrap_or_else(|| "".to_string()),
                        };
                        tx.send(chat_message).unwrap();
                    }
                    finish_reason = chat_choice.finish_reason.clone();
                });
            }
            Err(err) => {
                return Err(anyhow::Error::new(log_error!(err)));
            }
        }
    }
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
    tx: &'a mpsc::UnboundedSender<ChatMessage>,
    tool_calls: Vec<ChatCompletionMessageToolCall>,
) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + 'a>> {
    Box::pin(async move {
        // Your recursive logic here
        // For example, you can call process_user_message again
        process_user_message(prompt_engine, payload, tx, tool_calls).await
    })
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}
impl ToolCall {
    pub fn new(call_id: String) -> Self {
        Self {
            call_id,
            name: "".to_string(),
            arguments: "".to_string(),
        }
    }
}
