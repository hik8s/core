use std::{sync::Arc, time::Duration};

use shared::{
    connections::fluvio::offset::commit_and_flush_offsets,
    fluvio::TopicName,
    log_error_continue,
    types::{class::Class, tokenizer::Tokenizer},
    DbName, FluvioConnection, QdrantConnection, RateLimiter,
};

use tracing::info;

use crate::{error::DataVectorizationError, vectorize::vectorizer::vectorize_class_batch};

pub async fn vectorize_class(limiter: Arc<RateLimiter>) -> Result<(), DataVectorizationError> {
    let fluvio = FluvioConnection::new().await?;
    let qdrant = QdrantConnection::new().await?;
    let mut consumer = fluvio.create_consumer(0, TopicName::Class).await?;
    let tokenizer = Tokenizer::new()?;
    let polling_interval = Duration::from_millis(100);
    let db = DbName::Log;
    loop {
        // Accumulate batch
        let mut batch = fluvio.next_batch(&mut consumer, polling_interval).await?;

        // Process batch
        for (customer_id, records) in batch.drain() {
            let key = db.id(&customer_id);
            let classes: Vec<Class> = records
                .into_iter()
                .map(|record| record.try_into())
                .collect::<Result<Vec<Class>, _>>()
                .map_err(DataVectorizationError::ClassDeserialization)?;

            let (points, token_count) =
                log_error_continue!(vectorize_class_batch(&classes, &tokenizer, &limiter).await);

            log_error_continue!(qdrant.upsert_points(points, &db, &customer_id).await);

            info!(
                "Vectorized {} classes with {} tokens. Total used tokens: {}, ID: {}",
                classes.len(),
                token_count,
                limiter.tokens_used.lock().await,
                customer_id
            );
            log_error_continue!(commit_and_flush_offsets(&mut consumer, &key).await);
        }
    }
}
