use qdrant_client::qdrant::PointStruct;
use shared::{
    connections::openai::embeddings::request_embedding,
    types::{
        class::{
            vectorized::{to_qdrant_points, to_representations, to_vectorized_classes},
            Class,
        },
        tokenizer::Tokenizer,
    },
    utils::ratelimit::RateLimiter,
};

use crate::error::DataVectorizationError;

pub async fn vectorize_classes(
    classes: &[Class],
    tokenizer: &Tokenizer,
    rate_limiter: &RateLimiter,
) -> Result<(Vec<PointStruct>, usize), DataVectorizationError> {
    // Vectorize class
    let (vectorized_classes, total_token_count_cut) = to_vectorized_classes(classes, tokenizer);

    // Obey rate limit
    rate_limiter.check_rate_limit(total_token_count_cut).await;

    // Get embeddings
    let representations = to_representations(&vectorized_classes);
    let arrays = request_embedding(&representations).await?;

    // Create qdrant points
    let qdrant_points = to_qdrant_points(&vectorized_classes, &arrays)?;

    Ok((qdrant_points, total_token_count_cut))
}
