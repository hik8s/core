use fluvio::consumer::ConsumerStream;
use fluvio::dataplane::{link::ErrorCode, record::ConsumerRecord};
use fluvio::spu::SpuSocketPool;
use fluvio::TopicProducer;
use shared::connections::dbname::DbName;
use shared::connections::fluvio::util::get_record_key;
use shared::constant::TOPIC_CLASS_BYTES_PER_RECORD;
use shared::fluvio::{commit_and_flush_offsets, OffsetError};
use shared::preprocessing::log::preprocess_message;
use shared::{log_error, log_error_continue};

use std::str::Utf8Error;
use std::sync::Arc;

use futures_util::StreamExt;
use greptimedb_ingester::Error as GreptimeIngestError;
use shared::{
    connections::{
        greptime::{
            connect::{GreptimeConnection, GreptimeConnectionError},
            middleware::insert::classified_log_to_insert_request,
        },
        redis::connect::{RedisConnection, RedisConnectionError},
    },
    types::{
        classifier::error::ClassifierError,
        record::{log::LogRecord, preprocessed::PreprocessedLogRecord},
    },
};
use thiserror::Error;
use tracing::{error, warn};

use algorithm::classification::deterministic::classifier::Classifier;

#[derive(Error, Debug)]
pub enum ProcessThreadError {
    #[error("Classifier error: {0}")]
    ClassifierError(#[from] ClassifierError),
    #[error("Greptime connection error: {0}")]
    GreptimeConnectionError(#[from] GreptimeConnectionError),
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
    #[error("Stream inserter error: {0}")]
    StreamInserterError(#[from] GreptimeIngestError),
    #[error("Failed to serialize: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Fluvio producer error: {0}")]
    FluvioProducerError(#[from] anyhow::Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("Fluvio offset error: {0}")]
    OffsetError(#[from] OffsetError),
    #[error("Invalid json: {0}")]
    InvalidJson(String),
}

pub async fn process_logs(
    mut consumer: impl ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin,
    producer: Arc<TopicProducer<SpuSocketPool>>,
) -> Result<(), ProcessThreadError> {
    let redis = RedisConnection::new()?;
    let mut classifier = Classifier::new(None, redis)?;
    let greptime = GreptimeConnection::new().await?;
    while let Some(result) = consumer.next().await {
        let record = log_error_continue!(result);

        let customer_id = get_record_key(&record).map_err(|e| log_error!(e))?;
        let log = LogRecord::try_from(record)?;

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
            let serialized_record: String = class.try_into()?;
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
