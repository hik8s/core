use thiserror::Error;

use crate::ConfigError;

#[derive(Error, Debug)]
pub enum AuthenticationError {
    #[error("Config error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("JWT decode error: {0}")]
    JwtDecodeError(#[from] jsonwebtoken::errors::Error),
    #[error("Missing 'kid' in token header")]
    MissingKid,
    #[error("Key not found")]
    KeyNotFound,
    #[error("Token has expired")]
    TokenExpired,
    #[error("Fetch JWKS error: {0}")]
    FetchJwksError(#[from] Box<dyn std::error::Error>),
}
