use std::sync::Arc;

use shared::{connections::dbname::DbName, fluvio::TopicName, utils::ratelimit::RateLimiter};
use tokio::task::JoinHandle;

use crate::{
    error::DataVectorizationError, vectorize::vectorize_event::vectorize_event, vectorize_class,
    vectorize_resource,
};

pub fn run_vectorize_class(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    // Vector to hold all spawned threads
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        vectorize_class(limiter).await?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_vectorize_resource(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        let db = DbName::Resource;
        let topic = TopicName::ProcessedResource;
        vectorize_resource(limiter, db, topic).await?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_vectorize_customresource(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        let db = DbName::CustomResource;
        let topic = TopicName::ProcessedCustomResource;
        vectorize_resource(limiter, db, topic).await?;
        Ok(())
    }));

    Ok(threads)
}

pub fn run_vectorize_event(
    limiter: Arc<RateLimiter>,
) -> Result<Vec<JoinHandle<Result<(), DataVectorizationError>>>, DataVectorizationError> {
    let mut threads: Vec<JoinHandle<Result<(), DataVectorizationError>>> = Vec::new();

    threads.push(tokio::spawn(async move {
        let db = DbName::Event;
        let topic = TopicName::ProcessedEvent;
        vectorize_event(limiter, db, topic).await?;
        Ok(())
    }));

    Ok(threads)
}
