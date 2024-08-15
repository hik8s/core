use futures_util::StreamExt;
use logs::classification::Classifier;
use logs::types::appcontext::AppContext;
use logs::types::appstate::{AppState, AppStateError};
use serde_json::from_str;
use shared::connections::fluvio::connect::ConnectionError;
use shared::types::parsedline::ParsedLine;
use shared::{connections::fluvio::connect::FluvioConnection, tracing::setup::setup_tracing};
use thiserror::Error;
use tokio::task;
use tracing::error;

pub mod logs;

#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] ConnectionError),
    #[error("Other error: {0}")]
    Other(#[from] Box<dyn std::error::Error>),
    #[error("App state error: {0}")]
    AppStateError(#[from] AppStateError),
}

#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing();

    let fluvio_connection = FluvioConnection::new().await?;
    let mut consumer = fluvio_connection.create_consumer().await?;

    let state = AppState::new();
    let classifier = Classifier::new(None);

    while let Some(Ok(record)) = consumer.next().await {
        // let state = state.clone();
        let classifier = classifier.clone();
        let payload = record.value().to_vec();
        let key = record.key().map(|k| k.to_vec());

        let data_str = String::from_utf8_lossy(&payload);
        let key_str = String::from_utf8_lossy(&key.unwrap_or_default()).to_string();

        match from_str::<ParsedLine>(&data_str) {
            Ok(parsed_line) => match state.get_or_create(&key_str).await {
                Ok(app_context) => {
                    task::spawn(async move {
                        classifier.classify(parsed_line, app_context.app);
                    });
                }
                Err(e) => error!("Failed to get or create app context: {}", e),
            },
            Err(e) => error!("Failed to deserialize record: {}", e),
        }
    }
    Ok(())
}
