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
    x5t: String,
    x5c: Vec<String>,
}

async fn fetch_jwks(uri: &str) -> Result<Jwks, Box<dyn Error>> {
    let client = Client::builder().use_rustls_tls().build()?;
    let res = client.get(uri).send().await?;
    let jwks = res.json::<Jwks>().await?;
    Ok(jwks)
}

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use std::env;

use super::error::AuthenticationError;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub async fn validate_token(token: &str) -> Result<String, AuthenticationError> {
    let auth0_domain = env::var("AUTH0_DOMAIN")?;
    let jwks_uri = format!("https://{}/.well-known/jwks.json", auth0_domain);
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
    validation.set_audience(&[env::var("AUTH0_AUDIENCE")?]);
    validation.set_issuer(&[format!("https://{}/", auth0_domain)]);

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    if token_data.claims.exp > chrono::Utc::now().timestamp() as usize {
        let client_id = parse_client_id(&token_data.claims.sub);
        Ok(client_id.to_string())
    } else {
        Err(AuthenticationError::TokenExpired)
    }
}

fn parse_client_id(input: &str) -> &str {
    // TODO: sanitize input
    input.split('@').next().unwrap_or(input)
}
