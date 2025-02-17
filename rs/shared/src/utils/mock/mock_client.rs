use std::fmt::Display;

use rand::{distributions::Uniform, prelude::Distribution};
use rocket::{
    http::{ContentType, Header, Status},
    local::asynchronous::Client,
};

use crate::{get_env_var, types::metadata::Metadata};

pub async fn post_test_stream(client: &Client, route: &str, test_stream: String) -> Status {
    let token = get_env_var("AUTH_TOKEN").unwrap();
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

pub async fn post_test(client: &Client, route: &str, json_value: serde_json::Value) -> Status {
    let token = get_env_var("AUTH_TOKEN").unwrap();
    let response = client
        .post(route)
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .json(&json_value)
        .dispatch()
        .await;

    response.status()
}

pub async fn post_test_batch(
    client: &Client,
    route: &str,
    json_values: Vec<serde_json::Value>,
) -> Status {
    let token = get_env_var("AUTH_TOKEN").unwrap();
    let response = client
        .post(route)
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", token)))
        .json(&json_values) // Changed to send array of values
        .dispatch()
        .await;

    response.status()
}

fn generate_random_string(length: usize) -> String {
    let charset: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let mut rng = rand::thread_rng();
    let uniform = Uniform::from(0..charset.len());
    (0..length)
        .map(|_| charset[uniform.sample(&mut rng)])
        .collect()
}

pub fn generate_random_filename() -> String {
    let random_string = generate_random_string(6);
    format!("test-{}.txt", random_string)
}

pub fn generate_podname(case: impl Display) -> String {
    let part1 = generate_random_string(9);
    let part2 = generate_random_string(5);
    format!("{case}-{part1}-{part2}")
}

pub fn get_test_path(podname: &str) -> String {
    let base_path = "/var/log/pods";
    let pod_uid = "123-4123-53754";
    let namespace = format!("test-ns-{}", generate_random_string(5));
    let container = "some-container";
    format!("{base_path}/{namespace}_{podname}_{pod_uid}/{container}")
}

pub fn get_test_metadata(podname: &str) -> Metadata {
    let filename = generate_random_filename();
    let path = get_test_path(podname);
    Metadata::from_path(&filename, &path).unwrap()
}
