use futures_util::StreamExt;
use serde_json::{from_str, Value};
use shared::connections::dbname::DbName;
use shared::connections::fluvio::util::get_record_key;
use shared::connections::greptime::connect::GreptimeConnection;
use shared::connections::greptime::middleware::insert::resource_to_insert_request;

use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use shared::{log_error, log_error_continue};

use super::json_util::{
    extract_creation_timestamp, extract_managed_field_timestamps, get_as_option_string, get_as_ref,
    get_as_string,
};
use super::process::ProcessThreadError;

pub async fn process_resource(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin,
    db_name: DbName,
) -> Result<(), ProcessThreadError> {
    let greptime = GreptimeConnection::new().await?;

    while let Some(result) = consumer.next().await {
        let record = log_error_continue!(result);
        let customer_id = get_record_key(&record).map_err(|e| log_error!(e))?;

        greptime.create_database(&db_name, &customer_id).await?;

        let payload = record.value();
        let data_str = String::from_utf8_lossy(payload);
        let json: Value = from_str(&data_str).map_err(|e| log_error!(e))?;

        let kind = log_error_continue!(get_as_string(&json, "kind"));
        let apiversion = log_error_continue!(get_as_string(&json, "apiVersion"));

        let metadata = log_error_continue!(get_as_ref(&json, "metadata"));
        let uid = log_error_continue!(get_as_string(metadata, "uid"));
        let name = log_error_continue!(get_as_string(metadata, "name"));
        let namespace = get_as_option_string(metadata, "namespace");

        let mut timestamps = extract_managed_field_timestamps(metadata);
        timestamps.push(extract_creation_timestamp(metadata));
        timestamps.sort();
        let latest_timestamp = timestamps.last().unwrap_or(&0);

        let status = json.get("status").map(|s| s.to_string());
        let spec = json.get("spec").map(|s| s.to_string());

        let insert_request = resource_to_insert_request(
            apiversion,
            kind,
            name,
            uid,
            metadata.to_string(),
            namespace,
            spec,
            status,
            latest_timestamp.to_owned(),
        );
        let stream_inserter = greptime.streaming_inserter(&db_name, &customer_id)?;
        stream_inserter.insert(vec![insert_request]).await?;
        stream_inserter.finish().await?;
    }
    Ok(())
}
