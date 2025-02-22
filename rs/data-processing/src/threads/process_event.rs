use std::sync::Arc;

use fluvio::spu::SpuSocketPool;
use fluvio::TopicProducer;
use futures_util::StreamExt;
use shared::connections::dbname::DbName;
use shared::connections::fluvio::util::get_record_key;
use shared::connections::greptime::middleware::insert::resource_to_insert_request;

use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use shared::fluvio::commit_and_flush_offsets;
use shared::types::kubeapidata::KubeApiData;
use shared::{log_error, log_warn_continue, GreptimeConnection};

use shared::utils::{extract_timestamp, get_as_option_string, get_as_ref, get_as_string};

use super::error::ProcessThreadError;

pub async fn process_event(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>,
    producer: Arc<TopicProducer<SpuSocketPool>>,
    db_name: DbName,
) -> Result<(), ProcessThreadError> {
    let greptime = GreptimeConnection::new().await?;

    while let Some(result) = consumer.next().await {
        let record = log_warn_continue!(result);
        let customer_id = log_warn_continue!(get_record_key(&record));

        greptime.create_database(&db_name, &customer_id).await?;

        let data: KubeApiData = log_warn_continue!(record
            .try_into()
            .map_err(ProcessThreadError::DeserializationError));

        let event_type = log_warn_continue!(get_as_string(&data.json, "type"));
        if event_type == "Normal" {
            continue;
        }
        let apiversion = log_warn_continue!(get_as_string(&data.json, "apiVersion"));
        let last_timestamp = extract_timestamp(&data.json, "lastTimestamp");
        let message = get_as_option_string(&data.json, "message");
        let reason = get_as_option_string(&data.json, "reason");
        let resource = log_warn_continue!(get_as_ref(&data.json, "involvedObject"));
        let resource_name = get_as_option_string(resource, "name");
        let resource_kind = get_as_option_string(resource, "kind");
        let resource_uid = get_as_option_string(resource, "uid");
        let resource_namespace = get_as_option_string(resource, "namespace");

        let status = data.json.get("status").map(|s| s.to_string());
        let spec = data.json.get("spec").map(|s| s.to_string());

        let insert_request = resource_to_insert_request(
            apiversion,
            resource_kind,
            resource_name,
            resource_uid,
            None,
            resource_namespace,
            spec,
            status,
            reason,
            message,
            db_name.to_string(),
            last_timestamp,
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
            .map_err(|e| log_error!(e))
            .ok();
        producer.flush().await.map_err(|e| log_error!(e)).ok();

        commit_and_flush_offsets(&mut consumer, customer_id)
            .await
            .map_err(|e| log_error!(e))?;
    }
    Ok(())
}
