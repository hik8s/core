use std::sync::Arc;

use shared::{fluvio::TopicName, get_env_var_as_vec, log_error_with_message, DbName, RateLimiter};
use tokio::task::JoinHandle;

use crate::{
    error::DataVectorizationError, vectorize::vectorize_event::vectorize_event, vectorize_class,
    vectorize_resource,
};

pub fn run_vectorize_class(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        vectorize_class(limiter)
            .await
            .map_err(|e| log_error_with_message!("Class vectorizer thread exited with error", e))?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_vectorize_resource(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    let skiplist = get_env_var_as_vec("RESOURCE_SKIPLIST")?;
    threads.push(tokio::spawn(async move {
        let dbname = DbName::Resource;
        let topic = TopicName::ProcessedResource;
        // TODO process inital replicaset
        vectorize_resource(limiter, dbname, topic, skiplist)
            .await
            .map_err(|e| {
                log_error_with_message!("Resource vectorizer thread exited with error", e)
            })?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_vectorize_customresource(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    let skiplist = get_env_var_as_vec("CUSTOMRESOURCE_SKIPLIST")?;
    threads.push(tokio::spawn(async move {
        let dbname = DbName::CustomResource;
        let topic = TopicName::ProcessedCustomResource;
        vectorize_resource(limiter, dbname, topic, skiplist)
            .await
            .map_err(|e| {
                log_error_with_message!("Custom resource vectorizer thread exited with error", e)
            })?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_vectorize_event(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        let dbname = DbName::Event;
        let topic = TopicName::ProcessedEvent;
        vectorize_event(limiter, dbname, topic)
            .await
            .map_err(|e| log_error_with_message!("Event vectorizer thread exited with error", e))?;
        Ok(())
    }));

    Ok(threads)
}
