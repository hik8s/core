use std::sync::Arc;

use data_vectorizer::{error::DataVectorizationError, run::run_vectorize_class};
use shared::{
    constant::OPENAI_EMBEDDING_TOKEN_LIMIT, tracing::setup::setup_tracing,
    utils::ratelimit::RateLimiter,
};

#[tokio::main]
async fn main() -> Result<(), DataVectorizationError> {
    setup_tracing(true);
    let rate_limiter = Arc::new(RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT));

    run_vectorize_class(rate_limiter)?;
    Ok(())
}
