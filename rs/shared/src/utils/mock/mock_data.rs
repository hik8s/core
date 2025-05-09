use std::fmt::{Display, Formatter, Result};
use std::vec::IntoIter;

use rand::seq::SliceRandom;
use rand::thread_rng;
use uuid7::uuid7;

use crate::constant::{
    CONVERSION_BYTE_TO_MEBIBYTE, FLUVIO_BYTES_SAFTY_MARGIN, OPENAI_EMBEDDING_TOKEN_LIMIT,
    TOPIC_LOG_BYTES_PER_RECORD,
};
use crate::types::class::{item::Item, Class};
use crate::types::metadata::Metadata;

use super::mock_client::{generate_podname, get_test_metadata};

#[derive(Clone, Copy)]
pub enum TestCase {
    Simple,
    Short,
    DataIntakeLimit,
    DataProcessingLimit,
    OpenAiRateLimit,
}

impl Display for TestCase {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let name = match self {
            TestCase::Simple => "simple",
            TestCase::Short => "short",
            TestCase::DataIntakeLimit => "data-intake-limit",
            TestCase::DataProcessingLimit => "data-processing-limit",
            TestCase::OpenAiRateLimit => "openai-ratelimit",
        };
        write!(f, "test-{}", name)
    }
}

pub struct TestData {
    pub metadata: Metadata,
    pub raw_messages: Vec<String>,
    pub expected_class: Class,
}

pub struct TestDataIterator {
    metadata: Metadata,
    raw_iter: IntoIter<String>,
    class: Class,
}

impl IntoIterator for TestData {
    type Item = (Metadata, String, Class); // (key, raw_message, expected_class)
    type IntoIter = TestDataIterator;

    fn into_iter(self) -> Self::IntoIter {
        TestDataIterator {
            metadata: self.metadata,
            raw_iter: self.raw_messages.into_iter(),
            class: self.expected_class,
        }
    }
}

impl Iterator for TestDataIterator {
    type Item = (Metadata, String, Class); // (metadata, raw_message, expected_class)

    fn next(&mut self) -> Option<Self::Item> {
        match (self.raw_iter.next(), self.class.clone()) {
            (Some(raw_message), class) => Some((self.metadata.clone(), raw_message, class)),
            _ => None,
        }
    }
}

pub fn class_from_items(items: Vec<Item>, similarity: f64, metadata: &Metadata) -> Class {
    Class {
        length: items.len(),
        items,
        count: 1,
        class_id: uuid7().to_string(),
        similarity,
        key: metadata.pod_name.to_owned(),
        namespace: metadata.namespace.to_owned(),
        container: metadata.container.to_owned(),
        token_count: 0,
    }
}

pub fn get_test_data(case: TestCase) -> TestData {
    let metadata = get_test_metadata(&generate_podname(case));

    match case {
        TestCase::Simple => {
            let class = class_from_items(
                vec![
                    Item::Fix("INFO".to_string()),
                    Item::Fix("This".to_string()),
                    Item::Fix("is".to_string()),
                    Item::Fix("a".to_string()),
                    Item::Fix("test".to_string()),
                    Item::Fix("log".to_string()),
                    Item::Fix("line".to_string()),
                    Item::Var,
                ],
                0.875,
                &metadata,
            );
            TestData {
                raw_messages: vec![
                    get_test_line(1),
                    get_test_line(2),
                    get_test_line(3),
                    get_test_line(4),
                ],
                expected_class: class,
                metadata,
            }
        }
        TestCase::Short => {
            let metadata = get_test_metadata(&generate_podname(case));
            let raw_messages = vec![
                "Trace[535324451]: [878.754588ms] [878.754588ms] END".to_string(),
                "Trace[803048940]: [1h59m31.217943757s] [1h59m31.217943757s] END".to_string(),
                "Trace[2022532732]: [744.428697ms] [744.428697ms] END".to_string(),
                "Trace[2043416383]: [744.655377ms] [744.655377ms] END".to_string(),
            ];
            let items = vec![
                Item::Fix("Trace".to_string()),
                Item::Fix("[".to_string()),
                Item::Var,
                Item::Fix("]".to_string()),
                Item::Fix(":".to_string()),
                Item::Fix("[".to_string()),
                Item::Var,
                Item::Fix("]".to_string()),
                Item::Fix("[".to_string()),
                Item::Var,
                Item::Fix("]".to_string()),
                Item::Fix("END".to_string()),
            ];
            let expected_class = class_from_items(items, 0.75, &metadata);
            TestData {
                raw_messages,
                expected_class,
                metadata,
            }
        }
        TestCase::DataIntakeLimit => {
            let expected_class = generate_null_class(&metadata);
            let raw_messages = vec![generate_repeated_message(CONVERSION_BYTE_TO_MEBIBYTE)];
            TestData {
                raw_messages,
                expected_class,
                metadata,
            }
        }
        TestCase::DataProcessingLimit => {
            let expected_class = generate_null_class(&metadata);
            let mut message = generate_repeated_message_random(1024);
            message.truncate(TOPIC_LOG_BYTES_PER_RECORD - FLUVIO_BYTES_SAFTY_MARGIN);
            let raw_messages = vec![message];
            TestData {
                raw_messages,
                expected_class,
                metadata,
            }
        }
        TestCase::OpenAiRateLimit => {
            let mut raw_messages = Vec::new();
            let mut expected_class = generate_null_class(&metadata);
            expected_class.count = raw_messages.len() as u32; // this is a hack to avoid a vec of classes
            let max_token_count = 7000;
            let num_messages = OPENAI_EMBEDDING_TOKEN_LIMIT / max_token_count;
            for _ in 0..num_messages {
                let mut message = generate_repeated_message_random(1024);
                message.truncate(TOPIC_LOG_BYTES_PER_RECORD - FLUVIO_BYTES_SAFTY_MARGIN);
                raw_messages.push(message);
            }

            TestData {
                raw_messages,
                expected_class,
                metadata,
            }
        }
    }
}

fn get_test_line(n: u8) -> String {
    format!(
        "{ts} {m}\r",
        m = get_test_message(n),
        ts = get_test_timestamp(n)
    )
}

fn get_test_timestamp(n: u8) -> String {
    format!("2023-06-10T10:30:0{n}Z", n = n)
}

fn get_test_message(n: u8) -> String {
    // this message is 31 bytes long for n from [0,9]
    format!("INFO This is a test log line {n} ")
}

fn get_test_message_random(n: u8) -> String {
    let num = &n.to_string();
    let mut words = ["INFO", "This", "is", "a", "test", "log", "line", num];
    let mut rng = thread_rng();
    words.shuffle(&mut rng);
    let message = words.join(" ");
    format!("{message} ")
}

fn generate_repeated_message(r: usize) -> String {
    let timestamp = get_test_timestamp(1);
    let test_message = get_test_message(1);
    let repeated_message = test_message.repeat(r);
    format!("{} {}", timestamp, repeated_message)
}

fn generate_repeated_message_random(r: usize) -> String {
    let timestamp = get_test_timestamp(1);
    let mut repeated_message = String::new();

    for _ in 0..r {
        let test_message = get_test_message_random(1);
        repeated_message.push_str(&test_message);
    }

    format!("{} {}", timestamp, repeated_message)
}

fn generate_null_class(metadata: &Metadata) -> Class {
    class_from_items(vec![Item::Var], 0.0, metadata)
}
