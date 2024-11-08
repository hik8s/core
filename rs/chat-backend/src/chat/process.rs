use async_openai::types::{ChatCompletionRequestMessage, FinishReason};
use shared::{
    connections::{
        openai::{
            messages::{
                create_assistant_message, create_system_message, create_tool_message,
                create_user_message, extract_last_user_text_message,
            },
            tools::{collect_tool_call_chunks, Tool},
        },
        qdrant::connect::QdrantConnection,
        OpenAIConnection,
    },
    constant::OPENAI_CHAT_MODEL_MINI,
    log_error,
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
    qdrant: &QdrantConnection,
    messages: &mut Vec<ChatCompletionRequestMessage>,
    tx: &mpsc::UnboundedSender<String>,
    options: RequestOptions,
) -> Result<(), anyhow::Error> {
    let openai = OpenAIConnection::new();

    let user_message = extract_last_user_text_message(messages);
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

        if finish_reason == Some(FinishReason::Stop) {
            break;
        }

        let tool_calls = collect_tool_call_chunks(tool_call_chunks);
        if tool_calls.is_empty() {
            break;
        }

        let assistant_tool_request =
            create_assistant_message("Tool request", Some(tool_calls.clone()));
        messages.push(assistant_tool_request);
        for tool_call in tool_calls {
            let tool = Tool::try_from(tool_call.function).unwrap();
            let tool_output = tool
                .request(qdrant, &user_message, &options.client_id)
                .await
                .unwrap();
            let tool_submission = create_tool_message(&tool_output, &tool_call.id);
            messages.push(tool_submission);
        }
    }
    Ok(())
}
