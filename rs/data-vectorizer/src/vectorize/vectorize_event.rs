use std::{sync::Arc, time::Duration};

use shared::{
    connections::{dbname::DbName, qdrant::EventQdrantMetadata},
    fluvio::{commit_and_flush_offsets, TopicName},
    log_error_continue, log_warn_continue,
    types::{kubeapidata::KubeApiData, tokenizer::Tokenizer},
    utils::{get_as_option_string, get_as_ref, get_as_string, ratelimit::RateLimiter},
    FluvioConnection, QdrantConnection,
};

use crate::error::DataVectorizationError;

use super::vectorizer::vectorize_chunk;

pub async fn vectorize_event(
    limiter: Arc<RateLimiter>,
    db: DbName,
    topic: TopicName,
) -> Result<(), DataVectorizationError> {
    let fluvio = FluvioConnection::new().await?;
    let qdrant = QdrantConnection::new().await?;
    let mut consumer = fluvio.create_consumer(0, topic).await?;
    let tokenizer = Tokenizer::new()?;
    let polling_interval = Duration::from_millis(10);
    loop {
        // Accumulate batch
        let mut batch = fluvio.next_batch(&mut consumer, polling_interval).await?;

        // Process batch
        for (customer_id, records) in batch.drain() {
            // tracing::info!("ID {} | resources: {}", customer_id, records.len());

            let mut chunk = vec![];
            let mut metachunk = vec![];

            let mut total_token_count = 0;
            for record in records {
                let mut kube_api_data: KubeApiData = log_warn_continue!(record
                    .try_into()
                    .map_err(DataVectorizationError::DeserializationError));

                // let last_timestamp = extract_timestamp(&kube_api_data.json, "lastTimestamp");
                let message = get_as_option_string(&kube_api_data.json, "message");
                let reason = get_as_option_string(&kube_api_data.json, "reason");
                let event_type = get_as_option_string(&kube_api_data.json, "type");

                let resource =
                    log_warn_continue!(get_as_ref(&kube_api_data.json, "involvedObject"));
                let resource_apiversion = log_warn_continue!(get_as_string(resource, "apiVersion"));
                let resource_name = log_warn_continue!(get_as_string(resource, "name"));
                let resource_namespace = get_as_option_string(resource, "namespace")
                    .unwrap_or("not_namespaced".to_string());
                let resource_kind = log_warn_continue!(get_as_string(resource, "kind"));
                let resource_uid = log_warn_continue!(get_as_string(resource, "uid"));
                {
                    let metadata = kube_api_data
                        .json
                        .get_mut("metadata")
                        .expect("metadata field missing");

                    if let Some(metadata_obj) = metadata.as_object_mut() {
                        metadata_obj.remove("managedFields");
                    }
                }

                if let Ok(data) = serde_yaml::to_string(&kube_api_data.json) {
                    let (data_clip, token_count) = tokenizer.clip_tail(data.clone());
                    let resource_embedding = EventQdrantMetadata::new(
                        resource_apiversion,
                        resource_kind,
                        resource_uid,
                        resource_name,
                        resource_namespace,
                        message.unwrap_or_default(),
                        reason.unwrap_or_default(),
                        event_type.unwrap_or_default(),
                        data,
                    );
                    chunk.push(data_clip);
                    metachunk.push(resource_embedding);
                    total_token_count += token_count;
                }

                if total_token_count > 100000 {
                    limiter.check_rate_limit(total_token_count).await;
                    vectorize_chunk(
                        &mut chunk,
                        &mut metachunk,
                        &qdrant,
                        &customer_id,
                        &db,
                        total_token_count,
                    )
                    .await;
                    total_token_count = 0;
                }
            }

            limiter.check_rate_limit(total_token_count).await;
            vectorize_chunk(
                &mut chunk,
                &mut metachunk,
                &qdrant,
                &customer_id,
                &db,
                total_token_count,
            )
            .await;
            chunk.clear();
            metachunk.clear();
        }
        // commit fluvio offset
        log_error_continue!(commit_and_flush_offsets(&mut consumer, "".to_string()).await);
    }
}
