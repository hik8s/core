use rand::{distributions::Alphanumeric, Rng};

#[derive(Debug, Clone, Copy)]
pub enum TestCase {
    Simple,
}

pub fn generate_random_filename() -> String {
    let random_string: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    format!("test-{}.txt", random_string)
}

pub fn generate_podname(case: TestCase) -> String {
    let rng = rand::thread_rng();
    let part1: String = rng
        .clone()
        .sample_iter(&Alphanumeric)
        .take(9)
        .map(char::from)
        .collect();
    let part2: String = rng
        .sample_iter(&Alphanumeric)
        .take(5)
        .map(char::from)
        .collect();

    format!("{case:?}-{part1}-{part2}")
}

pub fn get_test_path() -> String {
    format!(
        "/var/log/pods/test-ns_{}_123-4123-53754/some-container",
        generate_podname(TestCase::Simple)
    )
}
