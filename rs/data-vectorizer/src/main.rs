use data_vectorizer::{error::DataVectorizationError, run::run_data_vectorizer};
use shared::tracing::setup::setup_tracing;

#[tokio::main]
async fn main() -> Result<(), DataVectorizationError> {
    setup_tracing(false);
    run_data_vectorizer().await
}
