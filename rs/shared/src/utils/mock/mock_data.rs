use std::fmt;

use rand::distributions::{Distribution, Uniform};

#[derive(Clone, Copy)]
pub enum TestCase {
    Simple,
}

impl fmt::Display for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            TestCase::Simple => "simple",
        };
        write!(f, "{}", name)
    }
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
