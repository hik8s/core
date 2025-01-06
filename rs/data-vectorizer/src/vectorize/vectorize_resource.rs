use std::{collections::HashMap, sync::Arc, time::Duration};

use serde::Serialize;
use serde_json::{from_str, Value};
use shared::{
    connections::{
        dbname::DbName,
        openai::embeddings::request_embedding,
        qdrant::{
            connect::{update_deleted_resources, QdrantConnection},
            ResourceQdrantMetadata,
        },
    },
    fluvio::{commit_and_flush_offsets, FluvioConnection, TopicName},
    log_error, log_error_continue,
    types::{
        class::vectorized::{to_qdrant_points, Id},
        kubeapidata::KubeApiData,
        tokenizer::Tokenizer,
    },
    utils::{
        create_metadata_map, extract_remove_key, get_as_option_string, get_as_string,
        ratelimit::RateLimiter,
    },
};
use tracing::{debug, info};

use crate::error::DataVectorizationError;

pub async fn vectorize_resource(
    limiter: Arc<RateLimiter>,
    db: DbName,
    topic: TopicName,
) -> Result<(), DataVectorizationError> {
    let fluvio = FluvioConnection::new().await?;
    let qdrant = QdrantConnection::new().await?;
    let mut consumer = fluvio.create_consumer(0, topic).await?;
    let tokenizer = Tokenizer::new()?;
    let polling_interval = Duration::from_millis(100);
    loop {
        // Accumulate batch
        let mut batch = fluvio.next_batch(&mut consumer, polling_interval).await?;

        // Process batch
        for (customer_id, records) in batch.drain() {
            // tracing::info!("ID {} | resources: {}", customer_id, records.len());

            let mut chunk: Vec<String> = vec![];
            let mut metachunk: Vec<ResourceQdrantMetadata> = vec![];
            let mut uids_deleted: Vec<String> = vec![];

            let mut total_token_count = 0;
            for record in records {
                let mut data: KubeApiData = log_error_continue!(record
                    .try_into()
                    .map_err(DataVectorizationError::DeserializationError));
                let kind = log_error_continue!(get_as_string(&data.json, "kind"));
                let metadata = data.json.get_mut("metadata").expect("metadata not found");

                let name = log_error_continue!(get_as_string(metadata, "name"));
                let uid = log_error_continue!(get_as_string(metadata, "uid"));
                let deletion_ts = get_as_option_string(metadata, "deletionTimestamp");

                if let Some(deletion_ts) = deletion_ts {
                    if !deletion_ts.is_empty() {
                        uids_deleted.push(uid.clone());
                        continue;
                    }
                }
                if data.event_type == "delete" {
                    uids_deleted.push(uid.clone());
                    continue;
                }
                let namespace = get_as_option_string(metadata, "namespace")
                    .unwrap_or("not_namespaced".to_string());

                if let Some(metadata_obj) = metadata.as_object_mut() {
                    metadata_obj.remove("managedFields");
                }

                let metadata_map = create_metadata_map(&name, &namespace, &uid);

                let spec = extract_remove_key(&mut data.json, &kind, &metadata_map, "spec");
                let status = extract_remove_key(&mut data.json, &kind, &metadata_map, "status");
                let metadata = serde_yaml::to_string(&data.json);

                debug!("MARKER\n{:?}{:?}{:?}", metadata, spec, status);
                let mut data_map = HashMap::new();
                if let Ok(data) = metadata {
                    data_map.insert("metadata", data);
                }
                if let Some(data) = spec {
                    data_map.insert("spec", data);
                }
                if let Some(data) = status {
                    data_map.insert("status", data);
                }
                for (key, data) in data_map {
                    let (data_clip, token_count) = tokenizer.clip_tail(data.clone());
                    let resource_embedding = ResourceQdrantMetadata::new(
                        kind.clone(),
                        uid.clone(),
                        name.clone(),
                        namespace.clone(),
                        data,
                        key.to_string(),
                    );
                    chunk.push(data_clip);
                    metachunk.push(resource_embedding);
                    total_token_count += token_count;
                }

                if total_token_count > 100000 {
                    limiter.check_rate_limit(total_token_count).await;
                    let chunk_len =
                        vectorize_chunk(&mut chunk, &mut metachunk, &qdrant, &customer_id, &db)
                            .await?;
                    info!("Vectorized {chunk_len} {db} with {total_token_count} tokens. ID: {customer_id}");
                    total_token_count = 0;
                }
            }

            limiter.check_rate_limit(total_token_count).await;
            let chunk_len =
                vectorize_chunk(&mut chunk, &mut metachunk, &qdrant, &customer_id, &db).await?;
            info!("Vectorized {chunk_len} {db} with {total_token_count} tokens. ID: {customer_id}");
            update_deleted_resources(&qdrant, &customer_id, &db, &uids_deleted).await?;

            chunk.clear();
            metachunk.clear();
            uids_deleted.clear();
        }
        // commit fluvio offset
        commit_and_flush_offsets(&mut consumer, "".to_string()).await?;
    }
    // Ok(())
}

async fn vectorize_chunk<T: Serialize + Id>(
    chunk: &mut Vec<String>,
    metachunk: &mut Vec<T>,
    qdrant: &QdrantConnection,
    customer_id: &str,
    db: &DbName,
) -> Result<usize, DataVectorizationError> {
    let arrays = request_embedding(chunk).await?;
    let qdrant_points = to_qdrant_points(metachunk, &arrays)?;
    qdrant.upsert_points(qdrant_points, db, customer_id).await?;
    let chunk_len = chunk.len();
    chunk.clear();
    metachunk.clear();
    Ok(chunk_len)
}
