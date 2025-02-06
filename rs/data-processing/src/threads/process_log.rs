use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use fluvio::spu::SpuSocketPool;
use fluvio::TopicProducer;
use shared::connections::dbname::DbName;
use shared::connections::fluvio::util::get_record_key;
use shared::constant::TOPIC_CLASS_BYTES_PER_RECORD;
use shared::fluvio::commit_and_flush_offsets;
use shared::preprocessing::log::preprocess_message;
use shared::{log_error, log_warn_continue};

use std::sync::Arc;

use futures_util::StreamExt;
use shared::{
    connections::{
        greptime::{
            connect::GreptimeConnection, middleware::insert::classified_log_to_insert_request,
        },
        redis::connect::RedisConnection,
    },
    types::record::{log::LogRecord, preprocessed::PreprocessedLogRecord},
};
use tracing::warn;

use algorithm::classification::deterministic::classifier::Classifier;

use super::error::ProcessThreadError;

pub async fn process_logs(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>>,
    producer: Arc<TopicProducer<SpuSocketPool>>,
) -> Result<(), ProcessThreadError> {
    let redis = RedisConnection::new().map_err(ProcessThreadError::RedisInit)?;
    let mut classifier = Classifier::new(None, redis)?;
    let greptime = GreptimeConnection::new().await?;
    while let Some(result) = consumer.next().await {
        let record = log_warn_continue!(result);

        let customer_id = log_warn_continue!(get_record_key(&record));
        let log = log_warn_continue!(
            LogRecord::try_from(record).map_err(ProcessThreadError::DeserializationError)
        );

        // preprocess
        let preprocessed_message =
            preprocess_message(&log.message, &customer_id, &log.key, &log.record_id);
        let preprocessed_log = PreprocessedLogRecord::from((log, preprocessed_message));

        // classify
        let (updated_class, classified_log) =
            classifier.classify(&preprocessed_log, &customer_id)?;

        // insert into greptimedb
        let insert_request = classified_log_to_insert_request(classified_log);
        let stream_inserter = greptime.streaming_inserter(&DbName::Log, &customer_id)?;
        stream_inserter
            .insert(vec![insert_request])
            .await
            .map_err(|e| log_error!(e))?;

        // produce to fluvio
        if updated_class.is_some() {
            let class = updated_class.unwrap();
            let key = class.key.clone();
            let class_id = class.class_id.clone();
            let serialized_record: String = log_warn_continue!(class
                .try_into()
                .map_err(ProcessThreadError::SerializationError));
            // TODO: add truncate for class.items
            if serialized_record.len() > TOPIC_CLASS_BYTES_PER_RECORD {
                warn!(
                    "Data too large for record, will be skipped. customer_id: {}, key: {}, record_id: {}, len: {}",
                    customer_id,
                    key,
                    class_id,
                    serialized_record.len()
                );
                continue;
            }
            // TODO: tolerate error and log with customer_id, key, record_id
            producer
                .send(customer_id.clone(), serialized_record)
                .await
                .map_err(|e| log_error!(e))?;
            producer.flush().await.map_err(|e| log_error!(e))?;
        }

        // commit consumed offsets
        commit_and_flush_offsets(&mut consumer, customer_id)
            .await
            .map_err(|e| log_error!(e))?;
    }
    Ok(())
}
