use rocket::{main, routes, Error};
use route::prompt::route::prompt_engine;
use shared::{
    connections::qdrant::connect::QdrantConnection, constant::QDRANT_COLLECTION_LOG,
    router::rocket::build_rocket, tracing::setup::setup_tracing,
};

pub mod route;

#[main]
async fn main() -> Result<(), Error> {
    setup_tracing();

    let qdrant_connection = QdrantConnection::new(QDRANT_COLLECTION_LOG.to_owned())
        .await
        .unwrap();

    let rocket = build_rocket(&vec![qdrant_connection], routes![prompt_engine]);

    rocket.launch().await?;
    Ok(())
}
