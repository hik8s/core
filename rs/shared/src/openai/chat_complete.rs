use crate::connections::prompt_engine::connect::PromptEngineConnection;
use crate::constant::OPENAI_CHAT_MODEL;
use async_openai::types::ChatCompletionRequestMessage;
use async_openai::{types::CreateChatCompletionRequestArgs, Client};
use futures_util::StreamExt;
use rocket::FromForm;
use serde::Serialize;
pub type Result<F, E = anyhow::Error> = anyhow::Result<F, E>;

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
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct Message {
    role: String,
    content: String,
}

pub async fn process_user_message<F>(
    prompt_engine: PromptEngineConnection,
    mut payload: RequestOptions,
    on_progress: F,
) -> Result<()>
where
    F: Fn(ChatMessage),
{
    // Enrich the latest user message with relevant context
    if let Some(user_message) = payload
        .messages
        .iter_mut()
        .rev()
        .find(|message| message.role == "user")
    {
        user_message.content = prompt_engine
            .request_augmentation(user_message.content.clone())
            .await?;
    }

    let messages: Vec<ChatCompletionRequestMessage> = payload
        .messages
        .into_iter()
        .map(|message| match message.role.as_str() {
            "system" => ChatCompletionRequestMessage::System(
                async_openai::types::ChatCompletionRequestSystemMessage {
                    content: "You are a site reliability engineer in a large company. You are in charge of troubleshooting a Kubernetes cluster.".to_string(),
                    name: Some("system".to_string()),
                },
            ),
            "user" => ChatCompletionRequestMessage::User(
                async_openai::types::ChatCompletionRequestUserMessage {
                    content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(
                        message.content,
                    ),
                    name: Some("username".to_string()),
                },
            ),
            #[allow(deprecated)]
            "assistant" => ChatCompletionRequestMessage::Assistant(
                async_openai::types::ChatCompletionRequestAssistantMessage {
                    content: Some(message.content),
                    name: Some("assistant".to_string()),
                    tool_calls: None,
                    function_call: None,
                },
            ),
            _ => panic!("Unknown role"),
        })
        .collect();

    let client = Client::new();

    let request = CreateChatCompletionRequestArgs::default()
        .model(OPENAI_CHAT_MODEL)
        .max_tokens(1024u16)
        .messages(messages)
        .build()?;

    let mut stream = client.chat().create_stream(request).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                response.choices.iter().for_each(|chat_choice| {
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
                        on_progress(chat_message);
                    }
                });
            }
            Err(err) => {
                return Err(anyhow::Error::new(err));
            }
        }
    }
    Ok(())
}
