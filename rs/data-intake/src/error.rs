use rocket::{http::Status, response::Responder, Request, Response};
use shared::{
    connections::greptime::connect::GreptimeConnectionError, fluvio::FluvioConnectionError,
};
use std::io::Cursor;
use thiserror::Error;
use tracing::error;

use crate::process::multipart::{MultipartMetadataError, MultipartStreamError};

#[derive(Error, Debug)]
pub enum DataIntakeError {
    #[error("Failed to stream data: {0}")]
    MultipartIoError(#[source] std::io::Error),
    #[error("Payload too large, limit is: {0}")]
    PayloadTooLarge(String),
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
    FluvioConnectionError(#[from] FluvioConnectionError),
    #[error("GreptimeDB connection error: {0}")]
    GreptimeError(#[from] GreptimeConnectionError),
    #[error("Rocket error: {0}")]
    RocketError(#[from] rocket::Error),
}

impl From<DataIntakeError> for Status {
    fn from(error: DataIntakeError) -> Self {
        match error {
            DataIntakeError::MultipartIoError(_) => Status::InternalServerError,
            DataIntakeError::PayloadTooLarge(e) => {
                error!("Payload too large, limit: {:?}", e);
                Status::PayloadTooLarge
            }
            DataIntakeError::ContentTypeBoundaryMissing => {
                error!("Content type boundary missing");
                Status::BadRequest
            }
            DataIntakeError::MultipartDataInvalid(e) => {
                error!("Multipart data invalid: {:?}", e);
                Status::BadRequest
            }
            DataIntakeError::MultipartNoFields => {
                error!("Multipart no fields");
                Status::BadRequest
            }
            DataIntakeError::MultipartMetadata(e) => {
                error!("Multipart metadata error: {:?}", e);
                Status::InternalServerError
            }
            DataIntakeError::MultipartStream(e) => {
                error!("Multipart stream error: {:?}", e);
                Status::InternalServerError
            }
            DataIntakeError::MetadataNone => {
                error!("Metadata none");
                Status::BadRequest
            }
            DataIntakeError::GreptimeIngestError(e) => {
                error!("Greptime ingest error: {:?}", e);
                Status::InternalServerError
            }
            DataIntakeError::InsertError(e) => {
                error!("Insert error: {:?}", e);
                Status::InternalServerError
            }
            DataIntakeError::FinishError(e) => {
                error!("Finish error: {:?}", e);
                Status::InternalServerError
            }
            DataIntakeError::FluvioConnectionError(e) => {
                error!("Fluvio connection error: {:?}", e);
                Status::InternalServerError
            }
            DataIntakeError::MultipartUnexpectedFieldName(e) => {
                error!("Multipart unexpected field name: {:?}", e);
                Status::BadRequest
            }
            DataIntakeError::GreptimeError(_) => Status::InternalServerError,
            DataIntakeError::RocketError(_) => Status::InternalServerError,
        }
    }
}

impl<'r> Responder<'r, 'static> for DataIntakeError {
    fn respond_to(self, _: &'r Request<'_>) -> Result<Response<'static>, Status> {
        let body = format!("{}", self);
        let status = Status::from(self);
        Response::build()
            .status(status)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}
