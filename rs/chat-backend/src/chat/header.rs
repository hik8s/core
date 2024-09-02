use rocket::http::Status;
use rocket::request::Outcome;
use rocket::request::Request;

pub struct LastEventId(pub usize);

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for LastEventId {
    type Error = std::num::ParseIntError;
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(id) = req.headers().get("Last-Event-ID").next() {
            if id.is_empty() {
                Outcome::Success(LastEventId(0))
            } else {
                match id.parse() {
                    Ok(id) => Outcome::Success(LastEventId(id)),
                    Err(err) => Outcome::Error((Status::BadRequest, err)),
                }
            }
        } else {
            Outcome::Success(LastEventId(0))
        }
    }
}
