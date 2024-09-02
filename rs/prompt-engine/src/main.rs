use prompt::route::prompt_engine;
use rocket::{main, routes, Error};
use shared::{
    connections::qdrant::connect::QdrantConnection,
    constant::{PROMPT_ENGINE_PORT, QDRANT_COLLECTION_LOG},
    router::rocket::build_rocket,
    tracing::setup::setup_tracing,
};

pub mod prompt;

#[main]
async fn main() -> Result<(), Error> {
    setup_tracing();
    std::env::set_var("ROCKET_PORT", PROMPT_ENGINE_PORT);

    let qdrant_connection = QdrantConnection::new(QDRANT_COLLECTION_LOG.to_owned())
        .await
        .unwrap();

    let rocket = build_rocket(&vec![qdrant_connection], routes![prompt_engine]);

    rocket.launch().await?;
    Ok(())
}
