use chat_backend::route::{route_add_cluster::add_cluster, route_chat_completion::chat_completion};
use rocket::{main, routes};
use shared::{
    constant::CHAT_BACKEND_PORT,
    router::rocket::{build_rocket, Connection},
    setup_tracing, GreptimeConnection, GreptimeConnectionError, QdrantConnection,
    QdrantConnectionError,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatBackendError {
    #[error("Greptime error: {0}")]
    Greptime(#[from] GreptimeConnectionError),
    #[error("Qdrant error: {0}")]
    Qdrant(#[from] QdrantConnectionError),
    #[error("Rocket error: {0}")]
    Rocket(#[from] rocket::Error),
}

#[main]
async fn main() -> Result<(), ChatBackendError> {
    setup_tracing(false);
    std::env::set_var("ROCKET_PORT", CHAT_BACKEND_PORT);

    let connections = [
        Connection::from(GreptimeConnection::new().await?),
        Connection::from(QdrantConnection::new().await?),
    ];

    let routes = routes![chat_completion, add_cluster];

    let rocket = build_rocket(&connections, routes);

    rocket.launch().await?;
    Ok(())
}
