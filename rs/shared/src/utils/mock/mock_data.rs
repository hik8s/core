use std::fmt::{Display, Formatter, Result};

use uuid7::uuid7;

use crate::types::classification::{class::Class, item::Item};

use super::mock_client::generate_podname;

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
    pub key: String,
    pub raw_messages: Vec<String>,
    pub expected_classes: Vec<Class>,
}

fn class_from_items(items: Vec<Item>, similarity: f64) -> Class {
    Class {
        length: items.len(),
        items,
        count: 1,
        class_id: uuid7().to_string(),
        similarity,
    }
}

pub fn get_test_data(case: TestCase) -> TestData {
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
            );
            TestData {
                key: generate_podname(case),
                raw_messages: vec![
                    "2023-06-10T10:30:01Z INFO This is a test log line 1\r".to_string(),
                    "2023-06-10T10:30:02Z INFO This is a test log line 2\r".to_string(),
                    "2023-06-10T10:30:03Z INFO This is a test log line 3\r".to_string(),
                    "2023-06-10T10:30:04Z INFO This is a test log line 4\r".to_string(),
                ],
                expected_classes: vec![class.clone(), class.clone(), class.clone(), class],
            }
        }
    }
}
