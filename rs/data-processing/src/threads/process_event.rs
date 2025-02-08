use std::sync::Arc;

use fluvio::spu::SpuSocketPool;
use fluvio::TopicProducer;
use futures_util::StreamExt;
use shared::connections::fluvio::util::get_record_key;

use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use shared::fluvio::commit_and_flush_offsets;
use shared::types::kubeapidata::KubeApiData;
use shared::{log_error, log_warn_continue};

use shared::utils::get_as_string;

use super::error::ProcessThreadError;

pub async fn process_event(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>,
    producer: Arc<TopicProducer<SpuSocketPool>>,
) -> Result<(), ProcessThreadError> {
    while let Some(result) = consumer.next().await {
        let record = log_warn_continue!(result);
        let customer_id = log_warn_continue!(get_record_key(&record));

        let data: KubeApiData = log_warn_continue!(record
            .try_into()
            .map_err(ProcessThreadError::DeserializationError));

        let event_type = log_warn_continue!(get_as_string(&data.json, "type"));
        if event_type == "Normal" {
            continue;
        }

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
