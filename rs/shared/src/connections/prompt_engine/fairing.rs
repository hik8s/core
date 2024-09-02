use rocket::{request::FromRequest, State};

use super::connect::PromptEngineConnection;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for PromptEngineConnection {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let connection = request
            .guard::<&State<PromptEngineConnection>>()
            .await
            .unwrap();
        rocket::request::Outcome::Success(connection.inner().clone())
    }
}

#[rocket::async_trait]
impl rocket::fairing::Fairing for PromptEngineConnection {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "Reqwest http client",
            kind: rocket::fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
        Ok(rocket.manage(self.clone()))
    }
}
