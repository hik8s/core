use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::StreamExt;
use qdrant_client::qdrant::PointStruct;
use shared::{
    connections::{
        dbname::DbName,
        fluvio::{offset::commit_and_flush_offsets, util::get_record_key},
        openai::embeddings::request_embedding,
        qdrant::connect::QdrantConnection,
    },
    fluvio::{FluvioConnection, TopicName},
    log_error, log_error_continue,
    types::{
        class::{
            vectorized::{to_qdrant_points, to_representations, to_vectorized_classes},
            Class,
        },
        tokenizer::Tokenizer,
    },
    utils::ratelimit::RateLimiter,
};
use tokio::time::timeout;
use tracing::info;

use crate::error::DataVectorizationError;

pub async fn vectorize_class(limiter: Arc<RateLimiter>) -> Result<(), DataVectorizationError> {
    let fluvio = FluvioConnection::new().await?;
    let qdrant = QdrantConnection::new().await?;
    let mut consumer = fluvio.create_consumer(0, TopicName::Class).await?;
    let tokenizer = Tokenizer::new()?;

    let polling_interval = Duration::from_millis(100);
    loop {
        let mut batch = HashMap::<String, Vec<Class>>::new();
        let start_time = tokio::time::Instant::now();

        // Accumulate batch
        while start_time.elapsed() < polling_interval {
            let result = match timeout(polling_interval, consumer.next()).await {
                Ok(Some(Ok(record))) => Ok(record),
                Ok(Some(Err(e))) => Err(e), // error receiving record
                Ok(None) => continue,       // consumer stream ended (does not happen)
                Err(_) => continue,         // no record received within the timeout
            };
            let record = log_error_continue!(result);
            let customer_id = get_record_key(&record).map_err(|e| log_error!(e))?;
            let class: Class = record.try_into()?;

            if let Some(classes) = batch.get_mut(&customer_id) {
                classes.push(class);
            } else {
                let mut classes = Vec::new();
                classes.push(class);
                batch.insert(customer_id, classes);
            }
        }

        // Process batch
        for (customer_id, classes) in batch.drain() {
            let (points, total_token_count) =
                vectorize_class_batch(&classes, &tokenizer, &limiter).await?;
            info!(
                "Vectorized {} classes with {total_token_count} tokens. Total used tokens: {}, ID: {}",
                classes.len(),
                limiter.tokens_used.lock().await,
                customer_id
            );
            qdrant
                .upsert_points(points, &DbName::Log, &customer_id)
                .await?;
        }
        // commit fluvio offset
        commit_and_flush_offsets(&mut consumer, "".to_string()).await?;
    }
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
    let qdrant_points = to_qdrant_points(&vectorized_classes, &arrays)?;

    Ok((qdrant_points, total_token_count_cut))
}
