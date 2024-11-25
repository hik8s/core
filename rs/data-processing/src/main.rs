use data_processing::error::DataProcessingError;
use data_processing::run::{
    run_customresource_processing, run_data_processing, run_event_processing,
    run_resource_processing,
};
use shared::tracing::setup::setup_tracing;

#[tokio::main]
async fn main() -> Result<(), DataProcessingError> {
    setup_tracing(true);
    let mut threads = run_data_processing()?;
    threads.extend(run_resource_processing()?);
    threads.extend(run_customresource_processing()?);

    threads.extend(run_event_processing()?);
    // Wait for all threads to complete
    for thread in threads {
        thread.await??;
    }
    Ok(())
}
