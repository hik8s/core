use std::fmt::{Display, Formatter, Result};
use std::vec::IntoIter;

use uuid7::uuid7;

use crate::types::classification::{class::Class, item::Item};
use crate::types::metadata::Metadata;

use super::mock_client::{generate_podname, get_test_metadata};

#[derive(Clone, Copy)]
pub enum TestCase {
    Simple,
}

impl Display for TestCase {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let name = match self {
            TestCase::Simple => "simple",
        };
        write!(f, "{}", name)
    }
}

pub struct TestData {
    pub metadata: Metadata,
    pub raw_messages: Vec<String>,
    pub expected_classes: Vec<Class>,
}

pub struct TestDataIterator {
    metadata: Metadata,
    raw_iter: IntoIter<String>,
    class_iter: IntoIter<Class>,
}

impl IntoIterator for TestData {
    type Item = (Metadata, String, Class); // (key, raw_message, expected_class)
    type IntoIter = TestDataIterator;

    fn into_iter(self) -> Self::IntoIter {
        if self.raw_messages.len() != self.expected_classes.len() {
            panic!("Failed into_iter for TestData: self.raw_messages and self.expected_classes must have the same length");
        }

        TestDataIterator {
            metadata: self.metadata,
            raw_iter: self.raw_messages.into_iter(),
            class_iter: self.expected_classes.into_iter(),
        }
    }
}

impl Iterator for TestDataIterator {
    type Item = (Metadata, String, Class); // (metadata, raw_message, expected_class)

    fn next(&mut self) -> Option<Self::Item> {
        match (self.raw_iter.next(), self.class_iter.next()) {
            (Some(raw_message), Some(expected_class)) => {
                Some((self.metadata.clone(), raw_message, expected_class))
            }
            _ => None,
        }
    }
}

fn class_from_items(items: Vec<Item>, similarity: f64, metadata: &Metadata) -> Class {
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
                    "2023-06-10T10:30:01Z INFO This is a test log line 1\r".to_string(),
                    "2023-06-10T10:30:02Z INFO This is a test log line 2\r".to_string(),
                    "2023-06-10T10:30:03Z INFO This is a test log line 3\r".to_string(),
                    "2023-06-10T10:30:04Z INFO This is a test log line 4\r".to_string(),
                ],
                expected_classes: vec![class.clone(), class.clone(), class.clone(), class],
                metadata,
            }
        }
    }
}
