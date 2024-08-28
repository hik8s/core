use futures_util::StreamExt;
use shared::{
    connections::{
        fluvio::connect::{
            commit_and_flush_offsets, FluvioConnection, FluvioConnectionError, OffsetError,
            TopicName,
        },
        qdrant::{connect::QdrantConnection, error::QdrantConnectionError},
    },
    constant::{OPENAI_EMBEDDING_TOKEN_LIMIT, QDRANT_COLLECTION_LOG},
    openai::embed::{request_embedding, RequestEmbeddingError},
    tracing::setup::setup_tracing,
    types::{
        classification::{class::Class, vectorized::to_qdrant_point},
        classifier::error::TokenizerError,
        record::consumer_record::ConsumerRecordError,
        tokenizer::tokenizer::Tokenizer,
    },
    utils::ratelimit::RateLimiter,
};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum DataVectorizationError {
    #[error("Fluvio connection error: {0}")]
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("Failed to parse fluvio consumer record: {0}")]
    ConsumerRecordError(#[from] ConsumerRecordError),
    #[error("Fluvio offset error: {0}")]
    FluvioOffsetError(#[from] OffsetError),
    #[error("Qdrant connection error: {0}")]
    QdrantConnectionError(#[from] QdrantConnectionError),
    #[error("Tokenizer error: {0}")]
    TokenizerError(#[from] TokenizerError),
    #[error("OpenAI API error: {0}")]
    OpenAIError(#[from] RequestEmbeddingError),
    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[tokio::main]
async fn main() -> Result<(), DataVectorizationError> {
    setup_tracing();
    let fluvio_connection = FluvioConnection::new(TopicName::Class).await?;
    let qdrant_connection = QdrantConnection::new(QDRANT_COLLECTION_LOG.to_owned()).await?;
    let mut consumer = fluvio_connection.create_consumer(0).await?;
    let tokenizer = Tokenizer::new()?;
    let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);
    while let Some(Ok(record)) = consumer.next().await {
        let class: Class = record.try_into()?;
        let (key, class_id) = (class.key.clone(), class.class_id.clone());

        // fit token limit
        let (representation, token_count) = tokenizer.clip_tail(class.to_string());

        // obey rate limit
        rate_limiter.check_rate_limit(token_count).await;

        // get embedding
        let embedding = request_embedding(representation.clone()).await?;

        // create qdrant point
        let qdrant_point = to_qdrant_point(class, token_count as u32, representation, embedding)?;

        // upsert to qdrant
        qdrant_connection.upsert_point(qdrant_point).await?;
        info!(
            "Successfully vectorized class with key: {}, id: {}",
            key, class_id
        );

        // commit fluvio offset
        commit_and_flush_offsets(&mut consumer, key, class_id).await?;
    }

    Ok(())
}
