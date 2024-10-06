use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthenticationError {
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),
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
