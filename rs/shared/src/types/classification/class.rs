use std::convert::TryInto;
use std::fmt::{self, Display, Formatter};

use fluvio::dataplane::record::ConsumerRecord;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use uuid7::uuid7;

use crate::types::record::consumer_record::{get_payload_and_key, ConsumerRecordError};
use crate::types::record::preprocessed::PreprocessedLogRecord;

use super::item::Item;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Class {
    pub items: Vec<Item>,
    pub count: i32,
    pub length: usize,
    pub class_id: String,
    pub similarity: f64,
    pub key: String,
}
impl Class {
    pub fn new(data: Vec<String>, key: &String) -> Self {
        let items: Vec<Item> = data.into_iter().map(Item::Fix).collect();
        Self {
            length: items.len(),
            items,
            count: 1,
            class_id: uuid7().to_string(),
            similarity: 0.0,
            key: key.to_owned(),
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

    pub fn from_log_and_key(log: &PreprocessedLogRecord, key: &String) -> Self {
        Self::new(log.preprocessed_message.clone(), key)
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
        tracing::info!("Deserializing class from: {}", data_str);
        from_str::<Class>(&data_str)
    }
}

impl TryFrom<ConsumerRecord> for Class {
    type Error = ConsumerRecordError;

    fn try_from(record: ConsumerRecord) -> Result<Self, Self::Error> {
        let (payload, _key) = get_payload_and_key(record)?;
        // key is also provided in payload
        // remove key retrieval once key is added in LogRecord
        let class: Class = payload.try_into()?;

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
