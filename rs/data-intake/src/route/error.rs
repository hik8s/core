use rocket::{http::Status, response::Responder, Request, Response};
use shared::connections::fluvio::connect::FluvioConnectionError;
use std::io::Cursor;
use thiserror::Error;

use crate::process::{metadata::MetadataError, multipart::MultipartProcessError};

#[derive(Error, Debug)]
pub enum LogIntakeError {
    #[error("Failed to stream data: {0}")]
    StreamDataError(#[source] std::io::Error),
    #[error("Missing boundary in content type")]
    MissingBoundary,
    #[error("Invalid multipart data: {0}")]
    InvalidMultipartData(#[source] std::io::Error),
    #[error("Metadata processing error: {0}")]
    MetadataProcessingError(#[source] MetadataError),
    #[error("Metadata is not set")]
    MetadataNotSet,
    #[error("Failed to get streaming inserter: {0}")]
    GreptimeIngestError(#[from] greptimedb_ingester::Error),
    #[error("Error during insert: {0}")]
    InsertError(#[source] std::io::Error),
    #[error("Error during finish: {0}")]
    FinishError(#[source] std::io::Error),
    #[error("Failed to send batch to Fluvio topic: {0}")]
    FluvioConnectionError(#[source] FluvioConnectionError),
    #[error("Unexpected field name: {0}")]
    UnexpectedFieldName(String),
    #[error("Expected field name: metadata or stream. No data was received.")]
    NoDataReceived,
    #[error("Error processing multipart: {0}")]
    MultipartProcessError(#[from] MultipartProcessError),
}

impl From<LogIntakeError> for Status {
    fn from(error: LogIntakeError) -> Self {
        match error {
            LogIntakeError::StreamDataError(_) => Status::PayloadTooLarge,
            LogIntakeError::MissingBoundary => Status::BadRequest,
            LogIntakeError::InvalidMultipartData(_) => Status::BadRequest,
            LogIntakeError::MetadataProcessingError(_) => Status::InternalServerError,
            LogIntakeError::MetadataNotSet => Status::BadRequest,
            LogIntakeError::GreptimeIngestError(_) => Status::InternalServerError,
            LogIntakeError::InsertError(_) => Status::InternalServerError,
            LogIntakeError::FinishError(_) => Status::InternalServerError,
            LogIntakeError::FluvioConnectionError(_) => Status::InternalServerError,
            LogIntakeError::UnexpectedFieldName(_) => Status::BadRequest,
            LogIntakeError::NoDataReceived => Status::BadRequest,
            LogIntakeError::MultipartProcessError(_) => Status::InternalServerError,
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
