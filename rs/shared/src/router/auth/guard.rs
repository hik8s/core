use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

use super::error::AuthenticationError;
use super::validation::validate_token;

#[derive(Debug)]
pub struct AuthenticatedUser {
    pub customer_id: String,
}

impl AuthenticatedUser {
    pub fn new(customer_id: String) -> Self {
        AuthenticatedUser { customer_id }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AuthenticationError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let keys: Vec<_> = request.headers().get("Authorization").collect();
        if keys.len() != 1 {
            return Outcome::Error((Status::Unauthorized, AuthenticationError::MissingKid));
        }

        let token = keys[0].trim_start_matches("Bearer ");
        match validate_token(token).await {
            Ok(customer_id) => Outcome::Success(AuthenticatedUser::new(customer_id)),
            Err(e) => {
                tracing::error!("Error validating token: {:?}", e);
                Outcome::Error((Status::Unauthorized, e))
            }
        }
    }
}
