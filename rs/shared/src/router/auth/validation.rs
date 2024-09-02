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

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub async fn validate_token(token: &str) -> Result<bool, Box<dyn Error>> {
    let auth0_domain = env::var("AUTH0_DOMAIN")?;
    let jwks_uri = format!("https://{}/.well-known/jwks.json", auth0_domain);
    let jwks = fetch_jwks(&jwks_uri).await?;

    let header = decode_header(token)?;
    let kid = header.kid.ok_or("Missing 'kid' in token header")?;

    let jwk = jwks.keys.into_iter().find(|key| key.kid == kid).ok_or("Key not found")?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[env::var("AUTH0_AUDIENCE")?]);
    validation.set_issuer(&[format!("https://{}/", auth0_domain)]);

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims.exp > chrono::Utc::now().timestamp() as usize)
}
