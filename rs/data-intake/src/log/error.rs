use rocket::{http::Status, response::Responder, Request, Response};
use shared::{
    connections::greptime::connect::GreptimeConnectionError, fluvio::FluvioConnectionError,
};
use std::io::Cursor;
use thiserror::Error;
use tracing::error;

use crate::process::multipart::{MultipartMetadataError, MultipartStreamError};

#[derive(Error, Debug)]
pub enum LogIntakeError {
    #[error("Failed to stream data: {0}")]
    PayloadTooLarge(#[source] std::io::Error),
    #[error("Missing boundary in content type")]
    ContentTypeBoundaryMissing,
    #[error("Invalid multipart data: {0}")]
    MultipartDataInvalid(#[source] std::io::Error),
    #[error("Multipart data missing")]
    MultipartNoFields,
    #[error("Error processing metadata: {0}")]
    MultipartMetadata(#[from] MultipartMetadataError),
    #[error("Error processing stream: {0}")]
    MultipartStream(#[from] MultipartStreamError),
    #[error("Unexpected field name: {0}")]
    MultipartUnexpectedFieldName(String),
    #[error("Metadata is not set")]
    MetadataNone,
    #[error("Failed to get streaming inserter: {0}")]
    GreptimeIngestError(#[from] greptimedb_ingester::Error),
    #[error("Error during insert: {0}")]
    InsertError(#[source] std::io::Error),
    #[error("Error during finish: {0}")]
    FinishError(#[source] std::io::Error),
    #[error("Failed to send batch to Fluvio topic: {0}")]
    FluvioConnectionError(#[source] FluvioConnectionError),
    #[error("GreptimeDB connection error: {0}")]
    GreptimeError(#[from] GreptimeConnectionError),
}

impl From<LogIntakeError> for Status {
    fn from(error: LogIntakeError) -> Self {
        match error {
            LogIntakeError::PayloadTooLarge(e) => {
                error!("Payload too large: {:?}", e);
                Status::PayloadTooLarge
            }
            LogIntakeError::ContentTypeBoundaryMissing => {
                error!("Content type boundary missing");
                Status::BadRequest
            }
            LogIntakeError::MultipartDataInvalid(e) => {
                error!("Multipart data invalid: {:?}", e);
                Status::BadRequest
            }
            LogIntakeError::MultipartNoFields => {
                error!("Multipart no fields");
                Status::BadRequest
            }
            LogIntakeError::MultipartMetadata(e) => {
                error!("Multipart metadata error: {:?}", e);
                Status::InternalServerError
            }
            LogIntakeError::MultipartStream(e) => {
                error!("Multipart stream error: {:?}", e);
                Status::InternalServerError
            }
            LogIntakeError::MetadataNone => {
                error!("Metadata none");
                Status::BadRequest
            }
            LogIntakeError::GreptimeIngestError(e) => {
                error!("Greptime ingest error: {:?}", e);
                Status::InternalServerError
            }
            LogIntakeError::InsertError(e) => {
                error!("Insert error: {:?}", e);
                Status::InternalServerError
            }
            LogIntakeError::FinishError(e) => {
                error!("Finish error: {:?}", e);
                Status::InternalServerError
            }
            LogIntakeError::FluvioConnectionError(e) => {
                error!("Fluvio connection error: {:?}", e);
                Status::InternalServerError
            }
            LogIntakeError::MultipartUnexpectedFieldName(e) => {
                error!("Multipart unexpected field name: {:?}", e);
                Status::BadRequest
            }
            LogIntakeError::GreptimeError(_) => Status::InternalServerError,
        }
    }
}

impl<'r> Responder<'r, 'static> for LogIntakeError {
    fn respond_to(self, _: &'r Request<'_>) -> Result<Response<'static>, Status> {
        let body = format!("{}", self);
        let status = Status::from(self);
        Response::build()
            .status(status)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}
