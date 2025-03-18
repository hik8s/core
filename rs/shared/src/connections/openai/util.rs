use async_openai::types::CreateChatCompletionStreamResponse;
use tokio::sync::mpsc;

pub async fn aggregate_answer(mut rx: mpsc::UnboundedReceiver<CreateChatCompletionStreamResponse>) -> String {
    let mut answer = String::new();
    while let Some(response) = rx.recv().await {
        if let Some(choice) = response.choices.first() {
            if let Some(ref message_delta) = choice.delta.content {
                answer.push_str(message_delta);
            }
        }
    }
    answer
}
