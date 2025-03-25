use rocket::get;
use rocket::serde::json::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct ClusterCredentials {
    client_id: String,
    client_secret: String,
}

#[get("/cluster/add")]
pub fn add_cluster() -> Json<ClusterCredentials> {
    Json(ClusterCredentials {
        client_id: "test-client-id".to_string(),
        client_secret: "test-client-secret".to_string(),
    })
}
