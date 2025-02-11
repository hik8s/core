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
use shared::types::kubeapidata::KubeApiData;
use shared::{log_error, log_error_continue, log_warn, log_warn_continue};

use super::error::ProcessThreadError;
use super::resource::process_deployment_conditions::{
    get_deployment_uid, update_deployment_conditions,
};
use super::resource::process_pod_conditions::{get_pod_key, update_pod_conditions};
use super::resource::update_state::update_resource_state;
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
            let mut new_state: Deployment = serde_json::from_value(data.json.clone())
                .map_err(ProcessThreadError::DeserializationError)?;
            new_state.metadata.managed_fields = None;
            let uid = log_error_continue!(get_deployment_uid(&new_state));
            let key = format!("{customer_id}:{kind}:{uid}");
            let requires_vectorization = log_error_continue!(
                update_resource_state(
                    &mut redis,
                    new_state,
                    &mut data,
                    &key,
                    update_deployment_conditions,
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
                    be retrieved from greptime
                - case: old entry in qdrant should be updated and not deleted. currently we would set
                    delete=true
            */

            let mut new_state: Pod = serde_json::from_value(data.json.clone())
                .map_err(ProcessThreadError::DeserializationError)?;
            new_state.metadata.managed_fields = None;
            let redis_uid = log_error_continue!(get_pod_key(&new_state));
            let key = format!("{customer_id}:{kind}:{redis_uid}");

            let requires_vectorization = log_error_continue!(
                update_resource_state(
                    &mut redis,
                    new_state,
                    &mut data,
                    &key,
                    update_pod_conditions,
                )
                .await
            );
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
