use std::sync::Arc;

use fluvio::spu::SpuSocketPool;
use fluvio::TopicProducer;
use futures_util::StreamExt;
use shared::connections::dbname::DbName;
use shared::connections::fluvio::util::get_record_key;

use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use shared::connections::greptime::greptime_connection::{parse_resource_name, GreptimeTable};
use shared::connections::greptime::middleware::insert::resource_to_insert_request;
use shared::constant::DEFAULT_NS;
use shared::fluvio::commit_and_flush_offsets;
use shared::types::kubeapidata::{KubeApiData, KubeEventType};
use shared::utils::{
    extract_managed_field_timestamps, extract_timestamp, get_as_ref, get_as_string,
};
use shared::{log_error_continue, log_warn, log_warn_continue, GreptimeConnection};

use crate::util::extract_metadata_owner::{extract_name_and_owner_name, extract_uid_and_owner_uid};

use super::error::ProcessThreadError;

pub async fn process_resource(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>,
    producer: Arc<TopicProducer<SpuSocketPool>>,
    dbname: DbName,
) -> Result<(), ProcessThreadError> {
    let greptime = GreptimeConnection::new().await?;
    while let Some(result) = consumer.next().await {
        let record = log_warn_continue!(result);
        let customer_id = log_warn_continue!(get_record_key(&record));
        let db = dbname.id(&customer_id);

        greptime.create_database(&db).await?;

        let data: KubeApiData = log_warn_continue!(record
            .try_into()
            .map_err(ProcessThreadError::DeserializationError));

        let kind = log_warn_continue!(get_as_string(&data.json, "kind"));

        let apiversion = log_warn_continue!(get_as_string(&data.json, "apiVersion"));

        let metadata = log_warn_continue!(get_as_ref(&data.json, "metadata"));
        let namespace = get_as_string(metadata, "namespace").unwrap_or(DEFAULT_NS.to_string());

        let mut timestamps = extract_managed_field_timestamps(metadata);
        timestamps.push(extract_timestamp(metadata, "creationTimestamp"));
        timestamps.sort();
        let latest_timestamp = timestamps.last().unwrap_or(&0);

        let status = data.json.get("status").map(|s| s.to_string());
        let spec = data.json.get("spec").map(|s| s.to_string());

        let (name, owner_name) = extract_name_and_owner_name(metadata);
        let (uid, owner_uid) = extract_uid_and_owner_uid(metadata);

        let table = greptime.create_table_name(&kind, &namespace, &owner_name, &owner_uid);

        let insert_request = resource_to_insert_request(
            apiversion,
            Some(kind.clone()),
            Some(name),
            Some(uid.clone()),
            Some(metadata.to_string()),
            Some(namespace),
            spec,
            status,
            None,
            None,
            table,
            latest_timestamp.to_owned(),
        );
        let stream_inserter = greptime.streaming_inserter(&db)?;
        stream_inserter.insert(vec![insert_request]).await?;
        stream_inserter.finish().await?;

        if data.event_type == KubeEventType::Delete {
            // TODO: handle errors
            let tables = greptime
                .list_tables(&db, Some(&uid), None, false)
                // this would cause a thread exit
                .await?
                .iter()
                .filter_map(|name| parse_resource_name(name))
                .collect::<Vec<GreptimeTable>>();

            for table in tables {
                if table.is_deleted {
                    continue;
                }
                greptime
                    .mark_table_deleted(&db, &table.format_name(false))
                    // this would cause a thread exit
                    .await?;
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

        log_error_continue!(commit_and_flush_offsets(&mut consumer).await);
    }
    Ok(())
}
