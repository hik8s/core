use std::{collections::HashMap, time::Duration};

use futures_util::StreamExt;
use qdrant_client::qdrant::PointStruct;
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
            vectorized::{to_qdrant_point, VectorizedClass},
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
            let (vectorized_classes, total_token_count_cut): (Vec<VectorizedClass>, u32) = classes
                .iter()
                .map(|class| class.vectorize(&tokenizer))
                .fold((Vec::new(), 0), |(mut vec, sum), vc| {
                    let token_count_cut = vc.token_count_cut;
                    vec.push(vc);
                    (vec, sum + token_count_cut)
                });
            for vectorized_class in &vectorized_classes {
                debug!(
                    "Vectorized class: {} with {} tokens",
                    vectorized_class.key, vectorized_class.token_count_cut
                );
            }

            // obey rate limit
            rate_limiter
                .check_rate_limit(total_token_count_cut as usize)
                .await;
            let representations = vectorized_classes
                .iter()
                .map(|vc| vc.representation.clone())
                .collect();

            // get embeddings
            let arrays = log_error_continue!(request_embedding(&representations).await);

            // create qdrant points
            let qdrant_points: Vec<PointStruct> = vectorized_classes
                .iter()
                .zip(arrays.iter())
                .map(|(vectorized_class, array)| to_qdrant_point(vectorized_class, *array))
                .collect::<Result<Vec<_>, _>>()?;

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
