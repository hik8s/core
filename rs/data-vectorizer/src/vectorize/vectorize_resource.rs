use std::{collections::HashMap, sync::Arc, time::Duration};

use shared::{
    connections::{
        dbname::DbName,
        qdrant::{
            connect::{update_deleted_resources, QdrantConnection},
            ResourceQdrantMetadata,
        },
    },
    fluvio::{commit_and_flush_offsets, FluvioConnection, TopicName},
    log_error_continue, log_warn_continue,
    types::{
        kubeapidata::{KubeApiData, KubeEventType},
        tokenizer::Tokenizer,
    },
    utils::{
        create_metadata_map, extract_remove_key, get_as_option_string, get_as_string,
        ratelimit::RateLimiter,
    },
};

use crate::{error::DataVectorizationError, vectorize::vectorizer::vectorize_chunk};

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
            let mut chunk: Vec<String> = vec![];
            let mut metachunk: Vec<ResourceQdrantMetadata> = vec![];
            let mut uids_deleted: Vec<String> = vec![];

            let mut total_token_count = 0;
            for record in records {
                let mut kube_api_data: KubeApiData = log_warn_continue!(record
                    .try_into()
                    .map_err(DataVectorizationError::DeserializationError));
                let kind = log_warn_continue!(get_as_string(&kube_api_data.json, "kind"));

                if kind.to_lowercase() == "replicaset" {
                    // TODO process inital replicaset
                    continue;
                }

                let mut requires_embedding = true;

                if kind.to_lowercase() == "pod" || kind.to_lowercase() == "deployment" {
                    requires_embedding = kube_api_data
                        .json
                        .get("status")
                        .and_then(|status| status.get("conditions"))
                        .and_then(|conditions| conditions.as_array())
                        .map(|conditions| {
                            conditions.iter().any(|condition| {
                                condition
                                    .get("status")
                                    .and_then(|status| status.as_str())
                                    .map(|s| s == "False")
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false);
                }

                let metadata = kube_api_data
                    .json
                    .get_mut("metadata")
                    .expect("metadata not found");

                let name = log_warn_continue!(get_as_string(metadata, "name"));
                let uid = log_warn_continue!(get_as_string(metadata, "uid"));

                if kube_api_data.event_type == KubeEventType::Delete {
                    uids_deleted.push(uid.clone());
                    continue;
                }

                if !requires_embedding {
                    continue;
                }

                let namespace = get_as_option_string(metadata, "namespace")
                    .unwrap_or("not_namespaced".to_string());

                if let Some(metadata_obj) = metadata.as_object_mut() {
                    metadata_obj.remove("managedFields");
                }

                let metadata_map = create_metadata_map(&name, &namespace, &uid);

                let spec =
                    extract_remove_key(&mut kube_api_data.json, &kind, &metadata_map, "spec");
                let status =
                    extract_remove_key(&mut kube_api_data.json, &kind, &metadata_map, "status");

                let remainder = serde_yaml::to_string(&kube_api_data.json);

                let mut data_map = HashMap::new();
                if let Ok(data) = remainder {
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

            log_error_continue!(
                update_deleted_resources(&qdrant, &customer_id, &db, &uids_deleted).await
            );

            chunk.clear();
            metachunk.clear();
            uids_deleted.clear();
        }
        // commit fluvio offset
        log_error_continue!(commit_and_flush_offsets(&mut consumer, "".to_string()).await);
    }
}
