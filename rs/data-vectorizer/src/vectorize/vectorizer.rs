use qdrant_client::qdrant::PointStruct;
use serde::Serialize;
use shared::{
    connections::openai::embeddings::request_embedding,
    log_error_with_message,
    types::{
        class::{
            vectorized::{to_qdrant_points, to_representations, to_vectorized_classes, Id},
            Class,
        },
        tokenizer::Tokenizer,
    },
    QdrantConnection, RateLimiter,
};
use tracing::info;

use crate::error::DataVectorizationError;

async fn try_vectorize_chunk<T: Serialize + Id>(
    chunk: &mut Vec<String>,
    metachunk: &mut Vec<T>,
    qdrant: &QdrantConnection,
    db: &str,
) -> Result<usize, DataVectorizationError> {
    let arrays = request_embedding(chunk).await?;
    let qdrant_points = to_qdrant_points(metachunk, &arrays)
        .map_err(DataVectorizationError::QdrantPointsConversion)?;
    qdrant.upsert_points(qdrant_points, db).await?;
    let chunk_len = chunk.len();
    chunk.clear();
    metachunk.clear();
    Ok(chunk_len)
}

pub async fn vectorize_chunk<T: Serialize + Id>(
    chunk: &mut Vec<String>,
    metachunk: &mut Vec<T>,
    qdrant: &QdrantConnection,
    db: &str,
    count: usize,
) {
    // unify chunk and metachunk
    if chunk.is_empty() {
        return;
    }
    match try_vectorize_chunk(chunk, metachunk, qdrant, db).await {
        Ok(chunk_len) => {
            info!("Vectorized {chunk_len} {db} with {count} tokens. ID: {db}")
        }
        Err(e) => {
            let message = format!("Failed to vectorize chunk for db: {db} with error");
            log_error_with_message!(message, e);
        }
    };
}

pub async fn vectorize_class_batch(
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
    let qdrant_points = to_qdrant_points(&vectorized_classes, &arrays)
        .map_err(DataVectorizationError::QdrantPointsConversion)?;

    Ok((qdrant_points, total_token_count_cut))
}
