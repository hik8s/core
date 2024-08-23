use std::fmt::{Display, Formatter, Result};

use uuid7::uuid7;

use crate::types::record::preprocessed::PreprocessedLogRecord;

use super::item::Item;

#[derive(Clone, Debug)]
pub struct Class {
    pub items: Vec<Item>,
    pub count: i32,
    pub length: usize,
    pub class_id: String,
    pub similarity: f64,
}
impl Class {
    pub fn new(data: Vec<String>) -> Self {
        let items: Vec<Item> = data.into_iter().map(Item::Fix).collect();
        Self {
            length: items.len(),
            items,
            count: 1,
            class_id: uuid7().to_string(),
            similarity: 0.0,
        }
    }

    pub fn mask_items(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|item| match item {
                Item::Fix(text) => text.clone(),
                _ => "".to_string(),
            })
            .collect()
    }

    pub fn update_items(&mut self, log: &PreprocessedLogRecord) {
        for (item, text) in self.items.iter_mut().zip(&log.preprocessed_message) {
            if let Item::Fix(item_str) = item {
                if text != item_str {
                    *item = Item::Var;
                }
            }
        }
    }
}

impl From<&PreprocessedLogRecord> for Class {
    fn from(log: &PreprocessedLogRecord) -> Self {
        Self::new(log.preprocessed_message.clone())
    }
}
impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let items = self
            .items
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        write!(f, "{items}")
    }
}
