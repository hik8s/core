use std::env::var;

use rocket::{
    http::{ContentType, Header, Status},
    local::asynchronous::Client,
};

pub async fn post_test_stream(client: &Client, route: &str, test_stream: String) -> Status {
    let token = var("AUTH0_TOKEN").expect("AUTH0_TOKEN must be set");
    let response = client
        .post(route)
        .header(
            ContentType::new("multipart", "form-data").with_params(vec![("boundary", "boundary")]),
        )
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .body(test_stream)
        .dispatch()
        .await;

    response.status()
}
