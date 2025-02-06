use std::sync::Arc;

use k8s_openapi::api::apps::v1::Deployment;

use fluvio::spu::SpuSocketPool;
use fluvio::TopicProducer;
use futures_util::StreamExt;
use shared::connections::dbname::DbName;
use shared::connections::fluvio::util::get_record_key;
use shared::connections::greptime::connect::GreptimeConnection;

use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use shared::connections::redis::connect::RedisConnection;
use shared::fluvio::commit_and_flush_offsets;
use shared::types::kubeapidata::KubeApiData;
use shared::{log_error, log_error_continue, log_warn, log_warn_continue};

use super::process::ProcessThreadError;
use super::resource::process_deployment::{
    get_conditions, get_conditions_len, unique_deployment_conditions,
};
use shared::utils::get_as_string;

pub async fn process_resource(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>,
    producer: Arc<TopicProducer<SpuSocketPool>>,
    db_name: DbName,
) -> Result<(), ProcessThreadError> {
    let greptime = GreptimeConnection::new().await?;
    let mut redis = RedisConnection::new().map_err(ProcessThreadError::RedisInit)?;

    while let Some(result) = consumer.next().await {
        let record = log_warn_continue!(result);
        let customer_id = log_warn_continue!(get_record_key(&record));

        greptime.create_database(&db_name, &customer_id).await?;

        let mut data: KubeApiData = log_warn_continue!(record
            .try_into()
            .map_err(ProcessThreadError::DeserializationError));

        let kind = log_warn_continue!(get_as_string(&data.json, "kind"));

        if kind == "Deployment" {
            let mut requires_vectorization = false;
            let mut incoming_resource: Deployment = serde_json::from_value(data.json.clone())
                .map_err(ProcessThreadError::DeserializationError)?;
            incoming_resource.metadata.managed_fields = None;
            let uid = log_error_continue!(incoming_resource
                .metadata
                .uid
                .to_owned()
                .ok_or(ProcessThreadError::MissingField("uid".to_string())));
            let key = format!("{customer_id}:{uid}");
            match log_error_continue!(redis
                .get_with_retry::<String>(&key)
                .await
                .map_err(ProcessThreadError::RedisGet))
            {
                None => {
                    // Serialize
                    let json = serde_json::to_string(&incoming_resource)
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
                    let init_num_conditions = get_conditions_len(&current_state);

                    let mut conditions = get_conditions(&current_state);
                    conditions.extend_from_slice(&get_conditions(&incoming_resource));
                    let aggregated_conditions = unique_deployment_conditions(conditions);

                    if let Some(status) = incoming_resource.status.as_mut() {
                        status.conditions = Some(aggregated_conditions)
                    }

                    let json = serde_json::to_string(&current_state)
                        .map_err(ProcessThreadError::SerializationError)?;
                    redis
                        .set_with_retry::<String>(&key, &json)
                        .await
                        .map_err(ProcessThreadError::RedisSet)?;

                    let new_num_conditions = get_conditions_len(&incoming_resource);
                    if new_num_conditions > init_num_conditions {
                        requires_vectorization = true;
                    }
                    data.json = serde_json::to_value(incoming_resource)
                        .map_err(ProcessThreadError::SerializationError)?;
                }
            }
            if !requires_vectorization {
                continue;
            }
        }

        let apiversion = log_warn_continue!(get_as_string(&data.json, "apiVersion"));

        let metadata = log_warn_continue!(get_as_ref(&data.json, "metadata"));
        let uid = get_as_option_string(metadata, "uid");
        let name = get_as_option_string(metadata, "name");
        let namespace = get_as_option_string(metadata, "namespace");

        let mut timestamps = extract_managed_field_timestamps(metadata);
        timestamps.push(extract_timestamp(metadata, "creationTimestamp"));
        timestamps.sort();
        let latest_timestamp = timestamps.last().unwrap_or(&0);

        let status = data.json.get("status").map(|s| s.to_string());
        let spec = data.json.get("spec").map(|s| s.to_string());

        let insert_request = resource_to_insert_request(
            apiversion,
            Some(kind.clone()),
            name,
            uid,
            Some(metadata.to_string()),
            namespace,
            spec,
            status,
            None,
            None,
            kind,
            latest_timestamp.to_owned(),
        );
        let stream_inserter = greptime.streaming_inserter(&db_name, &customer_id)?;
        stream_inserter.insert(vec![insert_request]).await?;
        stream_inserter.finish().await?;

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
