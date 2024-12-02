use std::sync::Arc;

use data_vectorizer::{
    error::DataVectorizationError,
    run::{
        run_vectorize_class, run_vectorize_customresource, run_vectorize_event,
        run_vectorize_resource,
    },
};
use shared::{
    constant::OPENAI_EMBEDDING_TOKEN_LIMIT, tracing::setup::setup_tracing,
    utils::ratelimit::RateLimiter,
};

#[tokio::main]
async fn main() -> Result<(), DataVectorizationError> {
    setup_tracing(true);
    let rate_limiter = Arc::new(RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT));

    let mut threads = run_vectorize_class(Arc::clone(&rate_limiter))?;
    threads.extend(run_vectorize_resource(Arc::clone(&rate_limiter))?);
    threads.extend(run_vectorize_customresource(Arc::clone(&rate_limiter))?);
    threads.extend(run_vectorize_event(rate_limiter)?);

    for thread in threads {
        thread.await??;
    }
    Ok(())
}
