use std::sync::Arc;

use k8s_openapi::api::apps::v1::Deployment;

use fluvio::spu::SpuSocketPool;
use fluvio::TopicProducer;
use futures_util::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use shared::connections::fluvio::util::get_record_key;

use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use shared::connections::redis::connect::RedisConnection;
use shared::fluvio::commit_and_flush_offsets;
use shared::types::kubeapidata::{KubeApiData, KubeEventType};
use shared::{log_error, log_error_continue, log_warn, log_warn_continue};

use super::error::ProcessThreadError;
use super::resource::process_deployment_conditions::update_deployment_conditions;
use super::resource::process_pod_conditions as pod;
use shared::utils::get_as_string;

pub async fn process_resource(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>,
    producer: Arc<TopicProducer<SpuSocketPool>>,
) -> Result<(), ProcessThreadError> {
    let mut redis = RedisConnection::new().map_err(ProcessThreadError::RedisInit)?;

    while let Some(result) = consumer.next().await {
        let record = log_warn_continue!(result);
        let customer_id = log_warn_continue!(get_record_key(&record));

        let mut data: KubeApiData = log_warn_continue!(record
            .try_into()
            .map_err(ProcessThreadError::DeserializationError));

        let kind = log_warn_continue!(get_as_string(&data.json, "kind"));

        if kind == "Deployment" {
            let mut requires_vectorization = false;
            let mut new_state: Deployment = serde_json::from_value(data.json.clone())
                .map_err(ProcessThreadError::DeserializationError)?;
            new_state.metadata.managed_fields = None;
            let uid = log_error_continue!(new_state
                .metadata
                .uid
                .to_owned()
                .ok_or(ProcessThreadError::MissingField("uid".to_string())));
            let key = format!("{customer_id}:{kind}:{uid}");
            match log_error_continue!(redis
                .get_with_retry::<String>(&key)
                .await
                .map_err(ProcessThreadError::RedisGet))
            {
                None => {
                    // Serialize
                    let json = serde_json::to_string(&new_state)
                        .map_err(ProcessThreadError::SerializationError)?;

                    // Set redis
                    redis
                        .set_with_retry::<String>(&key, &json)
                        .await
                        .map_err(ProcessThreadError::RedisSet)?;
                    requires_vectorization = true;
                }
                Some(json) => {
                    // update incoming resource from state
                    let current_state: Deployment =
                        log_error_continue!(serde_json::from_str(&json));
                    let (new_state, is_updated) =
                        update_deployment_conditions(current_state, new_state);

                    let json = serde_json::to_string(&new_state)
                        .map_err(ProcessThreadError::SerializationError)?;
                    redis
                        .set_with_retry::<String>(&key, &json)
                        .await
                        .map_err(ProcessThreadError::RedisSet)?;

                    if data.event_type == KubeEventType::Delete || is_updated {
                        // TODO: align with delete logic in data-vectorizer
                        requires_vectorization = true;
                    }
                    data.json = serde_json::to_value(new_state)
                        .map_err(ProcessThreadError::SerializationError)?;
                }
            }
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
                    be retrieved from greptime
                - case: old entry in qdrant should be updated and not deleted. currently we would set
                    delete=true
            */
            let mut requires_vectorization = false;
            let mut new_state: Pod = serde_json::from_value(data.json.clone())
                .map_err(ProcessThreadError::DeserializationError)?;
            new_state.metadata.managed_fields = None;

            let owner_uids = new_state.metadata.owner_references.as_ref().map(|refs| {
                refs.iter()
                    .map(|owner| owner.uid.as_ref())
                    .collect::<Vec<&str>>()
                    .join("_")
            });
            let redis_uid = if let Some(owner_uids) = owner_uids {
                owner_uids
            } else {
                log_error_continue!(new_state
                    .metadata
                    .uid
                    .to_owned()
                    .ok_or(ProcessThreadError::MissingField("uid".to_string())))
            };
            let key = format!("{customer_id}:{kind}:{redis_uid}");

            match log_error_continue!(redis
                .get_with_retry::<String>(&key)
                .await
                .map_err(ProcessThreadError::RedisGet))
            {
                None => {
                    // Serialize
                    let json = serde_json::to_string(&new_state)
                        .map_err(ProcessThreadError::SerializationError)?;

                    // Set redis
                    redis
                        .set_with_retry::<String>(&key, &json)
                        .await
                        .map_err(ProcessThreadError::RedisSet)?;
                    requires_vectorization = true;
                }
                Some(json) => {
                    // update incoming resource from state
                    let current_state: Pod = log_error_continue!(serde_json::from_str(&json));
                    let init_num_conditions = pod::get_conditions_len(&current_state);

                    let mut conditions = pod::get_conditions(&current_state);
                    conditions.extend_from_slice(&pod::get_conditions(&new_state));
                    let aggregated_conditions = pod::unique_conditions(conditions);

                    if let Some(status) = new_state.status.as_mut() {
                        status.conditions = Some(aggregated_conditions)
                    }

                    let json = serde_json::to_string(&new_state)
                        .map_err(ProcessThreadError::SerializationError)?;
                    redis
                        .set_with_retry::<String>(&key, &json)
                        .await
                        .map_err(ProcessThreadError::RedisSet)?;

                    let new_num_conditions = pod::get_conditions_len(&new_state);
                    if new_num_conditions > init_num_conditions {
                        requires_vectorization = true;
                    }

                    if data.event_type == KubeEventType::Delete {
                        // TODO: make more consistend
                        requires_vectorization = true;
                    }

                    data.json = serde_json::to_value(new_state)
                        .map_err(ProcessThreadError::SerializationError)?;
                }
            }
            if !requires_vectorization {
                continue;
            }
        }

        let data_serialized: Vec<u8> = log_warn_continue!(data
            .try_into()
            .map_err(ProcessThreadError::SerializationError));
        producer
            .send(customer_id.clone(), data_serialized)
            .await
            .map_err(|e| log_warn!(e))
            .ok();
        producer.flush().await.map_err(|e| log_warn!(e)).ok();

        commit_and_flush_offsets(&mut consumer, customer_id)
            .await
            .map_err(|e| log_error!(e))?;
    }
    Ok(())
}
