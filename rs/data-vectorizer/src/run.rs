use std::sync::Arc;

use shared::utils::ratelimit::RateLimiter;
use tokio::task::JoinHandle;

use crate::{error::DataVectorizationError, vectorize_class};

pub fn run_vectorize_class(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    // Vector to hold all spawned threads
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        vectorize_class(limiter).await?;
        Ok(())
    }));

    Ok(threads)
}
