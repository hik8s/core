use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

use super::validation::validate_token;

#[derive(Debug)]
pub struct AuthenticatedUser;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let keys: Vec<_> = request.headers().get("Authorization").collect();
        if keys.len() != 1 {
            return Outcome::Error((Status::Unauthorized, ()));
        }

        let token = keys[0].trim_start_matches("Bearer ");
        match validate_token(token).await {
            Ok(true) => Outcome::Success(AuthenticatedUser),
            _ => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
