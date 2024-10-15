use rocket::{local::asynchronous::Client, Build, Rocket};

pub async fn get_test_client(server: Rocket<Build>) -> Result<Client, rocket::Error> {
    dotenv::dotenv().ok();
    let client = Client::untracked(server).await?;
    Ok(client)
}
