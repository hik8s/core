use std::{collections::HashMap, sync::Arc, time::Duration};

use shared::{
    connections::qdrant::{connect::update_deleted_resources, ResourceQdrantMetadata},
    fluvio::{commit_and_flush_offsets, TopicName},
    log_error_continue, log_warn_continue,
    types::{
        kubeapidata::{KubeApiData, KubeEventType},
        tokenizer::Tokenizer,
    },
    utils::{
        create_metadata_map, extract_remove_key, get_as_option_string, get_as_string, get_uid,
        ratelimit::RateLimiter,
    },
    DbName, FluvioConnection, QdrantConnection, RedisConnection,
};

use crate::{
    error::DataVectorizationError,
    vectorize::{
        resource_state::{
            deployment_conditions::{
                get_deployment_uid, remove_deploy_managed_fields, update_deployment_conditions,
            },
            pod_conditions::{get_pod_key, remove_pod_managed_fields, update_pod_conditions},
            update_state::update_resource_state,
        },
        vectorizer::vectorize_chunk,
    },
};

pub async fn vectorize_resource(
    limiter: Arc<RateLimiter>,
    db: DbName,
    topic: TopicName,
    skiplist: Vec<String>,
) -> Result<(), DataVectorizationError> {
    let fluvio = FluvioConnection::new().await?;
    let qdrant = QdrantConnection::new().await?;
    let mut redis = RedisConnection::new().map_err(DataVectorizationError::RedisInit)?;
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

                let uid = log_warn_continue!(get_uid(&kube_api_data.json));
                if kube_api_data.event_type == KubeEventType::Delete {
                    uids_deleted.push(uid.clone());
                    continue;
                }
                let kind_lowercase = kind.to_lowercase();

                if skiplist.contains(&kind_lowercase) {
                    continue;
                }

                if kind == "Deployment" {
                    let requires_vectorization = log_error_continue!(
                        update_resource_state(
                            &customer_id,
                            &kind,
                            &mut redis,
                            &mut kube_api_data,
                            update_deployment_conditions,
                            get_deployment_uid,
                            remove_deploy_managed_fields
                        )
                        .await
                    );
                    if !requires_vectorization {
                        continue;
                    }
                }

                if kind == "Pod" {
                    /*
                    TODO:
                    - qdrant(payload): condition state updated or payload fields execpt data updated
                        - case: new pod without problems will have conditions with problems embeded.
                            That is ok, as the same replicaset had pod with problems. However, we must
                            provide the actual conditions of the pod to the model and should indicate
                            the problems of previous pods of that replicaset. this data should ideally
                            be retrieved from greptime.
                        - case: updated pods should have the latest pod uid to avoid mismatch of cluster
                            state and search space.
                    */

                    let requires_vectorization = log_error_continue!(
                        update_resource_state(
                            &customer_id,
                            &kind,
                            &mut redis,
                            &mut kube_api_data,
                            update_pod_conditions,
                            get_pod_key,
                            remove_pod_managed_fields
                        )
                        .await
                    );
                    if !requires_vectorization {
                        continue;
                    }
                }

                let metadata = kube_api_data
                    .json
                    .get_mut("metadata")
                    .expect("metadata not found");
                let resource_version =
                    get_as_string(metadata, "resourceVersion").unwrap_or("-1".to_string());

                let name = log_warn_continue!(get_as_string(metadata, "name"));

                let namespace = get_as_option_string(metadata, "namespace")
                    .unwrap_or("not_namespaced".to_string());

                if let Some(metadata_obj) = metadata.as_object_mut() {
                    metadata_obj.remove("managedFields");
                }

                let metadata_map = create_metadata_map(&name, &namespace, &uid, &resource_version);

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
