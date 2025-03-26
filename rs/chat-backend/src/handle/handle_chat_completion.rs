use async_openai::types::{
    ChatCompletionRequestMessage, CreateChatCompletionStreamResponse, FinishReason,
};
use shared::{
    connections::openai::messages::{create_final_message, create_iteration_loop_message},
    constant::DEFAULT_ITERATION_DEPTH,
    log_error,
    openai_util::{
        collect_tool_call_chunks, create_assistant_message, create_tool_message,
        extract_last_user_text_message, Tool,
    },
    GreptimeConnection, OpenAIConnection, QdrantConnection,
};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::{
    route::{error::ChatProcessingError, route_chat_completion::RequestOptions},
    util::tool_call_trace::ToolCallTrace,
};

pub async fn process_user_message(
    greptime: &GreptimeConnection,
    qdrant: &QdrantConnection,
    messages: &mut Vec<ChatCompletionRequestMessage>,
    tx: &mpsc::UnboundedSender<CreateChatCompletionStreamResponse>,
    options: RequestOptions,
) -> Result<(), ChatProcessingError> {
    let openai = OpenAIConnection::new();
    let user_message = extract_last_user_text_message(messages);
    let mut trace = ToolCallTrace::new(user_message.clone());
    tracing::info!("{}", trace.format_request());
    let max_depth = options.iteration_depth.unwrap_or(DEFAULT_ITERATION_DEPTH);
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
        let is_tool_called = !tool_calls.is_empty();

        let assistant_tool_request =
            create_assistant_message("Tool request", Some(tool_calls.clone()));
        messages.push(assistant_tool_request);
        for tool_call in tool_calls {
            let failure_id = tool_call.id.clone();

            let tool_output = match Tool::try_from(tool_call.function) {
                Ok(tool) => {
                    tracing::info!("{}", trace.format_tool_call(&tool));
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
        if is_tool_called {
            if trace.depth < max_depth - 1 {
                messages.push(create_iteration_loop_message(trace.depth));
            } else {
                messages.push(create_final_message(trace.depth));
            }
        }
        trace.depth += 1;
    }
    info!("{}", trace.format_final_message());
    Ok(())
}
