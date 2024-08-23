use std::env::var;

use rand::{distributions::Uniform, prelude::Distribution};
use rocket::{
    http::{ContentType, Header, Status},
    local::asynchronous::Client,
};

use super::mock_data::TestCase;

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

pub fn generate_podname(case: TestCase) -> String {
    let part1 = generate_random_string(9);
    let part2 = generate_random_string(5);
    format!("{case}-{part1}-{part2}")
}

pub fn get_test_path() -> String {
    let podname = generate_podname(TestCase::Simple);
    format!("/var/log/pods/test-ns_{podname}_123-4123-53754/some-container")
}
