use async_openai::types::{ChatCompletionRequestMessage, FinishReason, CreateChatCompletionStreamResponse};
use shared::{
    constant::OPENAI_CHAT_MODEL_MINI,
    log_error,
    openai_util::{
        collect_tool_call_chunks, create_assistant_message, create_system_message,
        create_tool_message, create_user_message, extract_last_user_text_message, Tool,
    },
    GreptimeConnection, OpenAIConnection, QdrantConnection,
};
use tokio::sync::mpsc;
use tracing::error;

use super::{error::ChatProcessingError, tool_call_trace::ToolCallTrace};

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
impl From<RequestOptions> for Vec<ChatCompletionRequestMessage> {
    fn from(val: RequestOptions) -> Self {
        val.messages
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
    greptime: &GreptimeConnection,
    qdrant: &QdrantConnection,
    messages: &mut Vec<ChatCompletionRequestMessage>,
    tx: &mpsc::UnboundedSender<CreateChatCompletionStreamResponse>,
    options: RequestOptions,
) -> Result<ToolCallTrace, ChatProcessingError> {
    let openai = OpenAIConnection::new();
    let user_message = extract_last_user_text_message(messages);
    let mut trace = ToolCallTrace::new(user_message.clone());
    loop {
        let request = openai.chat_complete_request(messages.clone(), &options.model, 1);
        let stream = openai
            .create_completion_stream(request)
            .await
            .map_err(|e| log_error!(e))?;
        let (finish_reason, tool_call_chunks) = openai
            .process_completion_stream(tx, stream)
            .await
            .map_err(|e| log_error!(e))?;

        if finish_reason == Some(FinishReason::Stop) && tool_call_chunks.is_empty() {
            break;
        }

        let tool_calls = collect_tool_call_chunks(tool_call_chunks)?;

        let assistant_tool_request =
            create_assistant_message("Tool request", Some(tool_calls.clone()));
        messages.push(assistant_tool_request);
        for tool_call in tool_calls {
            let failure_id = tool_call.id.clone();

            let tool_output = match Tool::try_from(tool_call.function) {
                Ok(tool) => {
                    trace.add_tool(&tool);
                    let tool_output = tool
                        .request(greptime, qdrant, &user_message, &options.client_id)
                        .await;
                    let failure_message = format!(
                        "Failed to execute tool with ID: {failure_id}. Sorry for the inconvenience."
                    );

                    tool_output
                        .inspect_err(|e| error!("{failure_message} Error: '{e}'"))
                        .unwrap_or(failure_message)
                }
                Err(e) => {
                    let failure_message = format!(
                        "Failed to parse tool with ID: {failure_id}. Sorry for the inconvenience."
                    );
                    error!("{failure_message} Error: '{e}'");
                    failure_message
                }
            };

            let tool_submission = create_tool_message(&tool_output, &tool_call.id);
            messages.push(tool_submission);
        }
        trace.depth += 1;
    }
    Ok(trace)
}
