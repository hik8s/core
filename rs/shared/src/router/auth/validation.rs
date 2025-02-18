use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Jwk {
    alg: String,
    kty: String,
    r#use: String,
    n: String,
    e: String,
    kid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    x5t: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    x5c: Option<Vec<String>>,
}

async fn fetch_jwks(uri: &str) -> Result<Jwks, Box<dyn Error>> {
    let client = Client::builder().use_rustls_tls().build()?;
    let res = client.get(uri).send().await?;
    let jwks = res.json::<Jwks>().await?;
    Ok(jwks)
}

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

use crate::{get_env_var, get_env_var_as_vec};

use super::error::AuthenticationError;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub async fn validate_token(token: &str) -> Result<String, AuthenticationError> {
    let auth_domain = get_env_var("AUTH_DOMAIN")?;
    let jwks_uri = format!("https://{}/.well-known/jwks.json", auth_domain);
    let jwks = fetch_jwks(&jwks_uri).await?;

    let header = decode_header(token)?;
    let kid = header.kid.ok_or(AuthenticationError::MissingKid)?;

    let jwk = jwks
        .keys
        .into_iter()
        .find(|key| key.kid == kid)
        .ok_or(AuthenticationError::KeyNotFound)?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[format!("https://{}", auth_domain)]);
    validation.set_audience(&get_env_audience()?);

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    if token_data.claims.exp > chrono::Utc::now().timestamp() as usize {
        validate_client_id(token_data.claims.sub)
    } else {
        Err(AuthenticationError::TokenExpired)
    }
}

fn get_env_audience() -> Result<Vec<String>, AuthenticationError> {
    match get_env_var_as_vec("AUTH_AUDIENCE")? {
        Some(audience) => Ok(audience),
        None => Err(AuthenticationError::MissingAudience(
            "No audience values found in AUTH_AUDIENCE".to_string(),
        )),
    }
}

fn validate_client_id(sub: String) -> Result<String, AuthenticationError> {
    if !sub.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(AuthenticationError::InvalidClientIdFormat(
            "Client id, must contain only alphanumeric characters".to_string(),
        ));
    }
    Ok(sub)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_subject() {
        // Valid cases
        assert!(validate_client_id("abc123ABC".to_owned()).is_ok());
        assert!(validate_client_id("59a98f41e8d14913b398cd7e4414c05e".to_owned()).is_ok());

        // Invalid cases
        assert!(validate_client_id("36afb309-638e-40fc-ae4e-c329219ed515".to_owned()).is_err());
        assert!(validate_client_id("abc-123".to_owned()).is_err());
        assert!(validate_client_id("abc@123".to_owned()).is_err());
        assert!(validate_client_id("abc 123".to_owned()).is_err());
    }
}
