use data_processing::error::DataProcessingError;
use data_processing::run::run_data_processing;
use shared::tracing::setup::setup_tracing;

#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing();
    run_data_processing().await?;
    Ok(())
}
