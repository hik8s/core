pub mod item;
pub mod vectorized;

use std::convert::TryInto;
use std::fmt::{self, Display, Formatter};

use fluvio::dataplane::record::ConsumerRecord;
use item::Item;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use uuid7::uuid7;

use crate::types::record::preprocessed::PreprocessedLogRecord;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Class {
    pub items: Vec<Item>,
    pub count: u32,
    pub length: usize,
    pub class_id: String,
    pub similarity: f64,
    pub token_count: u32,
    pub key: String,
    pub namespace: String,
    pub container: String,
}
impl Class {
    pub fn new(log: &PreprocessedLogRecord, token_count: u32) -> Self {
        let items: Vec<Item> = log
            .preprocessed_message
            .clone()
            .into_iter()
            .map(Item::Fix)
            .collect();
        Self {
            length: items.len(),
            items,
            count: 1,
            class_id: uuid7().to_string(),
            similarity: 0.0,
            token_count,
            key: log.key.to_owned(),
            namespace: log.namespace.to_owned(),
            container: log.container.to_owned(),
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

    pub fn from_log_and_token_count(log: &PreprocessedLogRecord, token_count: u32) -> Self {
        Self::new(log, token_count)
    }
}
impl TryInto<String> for Class {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        to_string(&self)
    }
}

impl TryFrom<Vec<u8>> for Class {
    type Error = serde_json::Error;

    // This is used for converting a fluvio ConsumerRecord into a Class
    fn try_from(payload: Vec<u8>) -> Result<Self, Self::Error> {
        let data_str = String::from_utf8_lossy(&payload);
        from_str::<Class>(&data_str)
    }
}

impl TryFrom<ConsumerRecord> for Class {
    type Error = serde_json::Error;

    fn try_from(record: ConsumerRecord) -> Result<Self, Self::Error> {
        let class: Class = record.value().to_vec().try_into()?;
        Ok(class)
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let items = self
            .items
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        write!(f, "{items}")
    }
}
