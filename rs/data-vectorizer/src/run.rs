use std::{collections::HashMap, time::Duration};

use futures_util::StreamExt;
use shared::{
    connections::{
        db_name::get_db_name,
        fluvio::{offset::commit_and_flush_offsets, util::get_record_key},
        qdrant::connect::QdrantConnection,
    },
    constant::OPENAI_EMBEDDING_TOKEN_LIMIT,
    fluvio::{FluvioConnection, TopicName},
    log_error, log_error_continue,
    openai::embed::request_embedding,
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
use tracing::debug;

use crate::error::DataVectorizationError;

pub async fn run_data_vectorizer() -> Result<(), DataVectorizationError> {
    let fluvio = FluvioConnection::new(TopicName::Class).await?;
    let qdrant = QdrantConnection::new().await?;
    let mut consumer = fluvio.create_consumer(0).await?;
    let tokenizer = Tokenizer::new()?;
    let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);
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
            // vectorize class
            let (vectorized_classes, total_token_count_cut) =
                to_vectorized_classes(&classes, &tokenizer);

            // obey rate limit
            rate_limiter.check_rate_limit(total_token_count_cut).await;

            // get embeddings
            let representations = to_representations(&vectorized_classes);
            let arrays = log_error_continue!(request_embedding(&representations).await);

            // create qdrant points
            let qdrant_points = to_qdrant_points(&vectorized_classes, &arrays)?;

            // upsert to qdrant
            let db_name = get_db_name(&customer_id);
            qdrant.create_collection(&db_name).await?;
            qdrant.upsert_points(qdrant_points, &db_name).await?;
            debug!(
                "Vectorized {} classes with {total_token_count_cut} tokens. Total used tokens: {}, ID: {}",
                vectorized_classes.len(),
                rate_limiter.tokens_used.lock().await,
                customer_id
            );
        }
        // commit fluvio offset
        commit_and_flush_offsets(&mut consumer, "".to_string()).await?;
    }
}
