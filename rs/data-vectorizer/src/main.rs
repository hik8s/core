use error::DataVectorizationError;
use run::run_data_vectorizer;
use shared::tracing::setup::setup_tracing;

pub mod error;
pub mod run;

#[tokio::main]
async fn main() -> Result<(), DataVectorizationError> {
    setup_tracing(false);
    run_data_vectorizer().await
}
