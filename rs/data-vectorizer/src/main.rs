use std::str::Utf8Error;

use futures_util::StreamExt;
use shared::{
    connections::{
        db_name::get_db_name,
        fluvio::{offset::commit_and_flush_offsets, util::get_record_key},
        qdrant::{connect::QdrantConnection, error::QdrantConnectionError},
    },
    constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
    fluvio::{FluvioConnection, FluvioConnectionError, OffsetError, TopicName},
    log_error,
    openai::embed::{request_embedding, RequestEmbeddingError},
    tracing::setup::setup_tracing,
    types::{
        class::{
            vectorized::{to_qdrant_point, VectorizedClass},
            Class,
        },
        tokenizer::Tokenizer,
    },
    utils::ratelimit::RateLimiter,
};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum DataVectorizationError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Fluvio offset error: {0}")]
    FluvioOffsetError(#[from] OffsetError),
    #[error("Qdrant connection error: {0}")]
    QdrantConnectionError(#[from] QdrantConnectionError),
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
    #[error("OpenAI API error: {0}")]
    OpenAIError(#[from] RequestEmbeddingError),
    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
}

#[tokio::main]
async fn main() -> Result<(), DataVectorizationError> {
    setup_tracing();
    let fluvio = FluvioConnection::new(TopicName::Class).await?;
    let qdrant = QdrantConnection::new().await?;
    let mut consumer = fluvio.create_consumer(0).await?;
    let tokenizer = Tokenizer::new()?;
    let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);
    while let Some(Ok(record)) = consumer.next().await {
        let customer_id = get_record_key(&record).map_err(|e| log_error!(e))?;

        let class: Class = record.try_into()?;
        let class_id = class.class_id.clone();

        // vectorize class
        let (representation, token_count) = tokenizer.clip_tail(class.to_string());
        let vectorized_class = VectorizedClass::new(class, token_count, representation.clone());

        // obey rate limit
        rate_limiter.check_rate_limit(token_count).await;

        // get embedding
        let array = request_embedding(&representation).await?;

        // create qdrant point
        let qdrant_point = to_qdrant_point(vectorized_class, array)?;

        // upsert to qdrant
        let db_name = get_db_name(&customer_id);
        qdrant.create_collection(&db_name).await?;
        qdrant.upsert_point(qdrant_point, &db_name).await?;
        info!(
            "Successfully vectorized class with key: {}, id: {}",
            customer_id, class_id
        );

        // commit fluvio offset
        commit_and_flush_offsets(&mut consumer, customer_id, class_id).await?;
    }

    Ok(())
}
