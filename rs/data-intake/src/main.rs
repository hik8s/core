use data_intake::{error::DataIntakeError, server::initialize_data_intake};
use shared::setup_tracing;

#[rocket::main]
async fn main() -> Result<(), DataIntakeError> {
    setup_tracing(false);

    let server = initialize_data_intake().await?;

    server.launch().await?;
    Ok(())
}
