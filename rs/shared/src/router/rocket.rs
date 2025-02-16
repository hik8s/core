use crate::{
    connections::greptime::connect::GreptimeConnection, fluvio::FluvioConnection, QdrantConnection,
};
use rocket::{catch, catchers, Build, Rocket, Route};

pub trait Attach: Clone {
    fn attach(self, rocket: Rocket<Build>) -> Rocket<Build>;
}

impl Attach for GreptimeConnection {
    fn attach(self, rocket: Rocket<Build>) -> Rocket<Build> {
        rocket.attach(self)
    }
}

impl Attach for FluvioConnection {
    fn attach(self, rocket: Rocket<Build>) -> Rocket<Build> {
        rocket.attach(self)
    }
}

impl Attach for QdrantConnection {
    fn attach(self, rocket: Rocket<Build>) -> Rocket<Build> {
        rocket.attach(self)
    }
}

#[derive(Clone)]
pub enum Connection {
    // we need this enum to provide a vec of connections to the build_rocket function
    // (vec must have the same type for all elements)
    Greptime(GreptimeConnection),
    Fluvio(FluvioConnection),
    Qdrant(QdrantConnection),
}

impl Attach for Connection {
    fn attach(self, rocket: Rocket<Build>) -> Rocket<Build> {
        match self {
            Connection::Greptime(conn) => conn.attach(rocket),
            Connection::Fluvio(conn) => conn.attach(rocket),
            Connection::Qdrant(conn) => conn.attach(rocket),
        }
    }
}

impl From<GreptimeConnection> for Connection {
    fn from(conn: GreptimeConnection) -> Self {
        Connection::Greptime(conn)
    }
}

impl From<FluvioConnection> for Connection {
    fn from(conn: FluvioConnection) -> Self {
        Connection::Fluvio(conn)
    }
}

impl From<QdrantConnection> for Connection {
    fn from(conn: QdrantConnection) -> Self {
        Connection::Qdrant(conn)
    }
}

pub fn build_rocket(connections: &[impl Attach], routes: Vec<Route>) -> Rocket<Build> {
    // mount routes
    let mut rocket = rocket::build()
        .mount("/", routes)
        .register("/", catchers![internal_error]);

    // attach connections
    for connection in connections {
        rocket = connection.clone().attach(rocket);
    }
    rocket
}

#[catch(500)]
fn internal_error() -> &'static str {
    // TODO: add a request id to trace errors
    "Internal Server Error. Please try again later."
}
