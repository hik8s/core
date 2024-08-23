use std::fmt::{Display, Formatter, Result};

use redis::{from_redis_value, FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};
use serde::{Deserialize, Serialize};
use uuid7::uuid7;

use crate::types::record::preprocessed::PreprocessedLogRecord;

use super::item::Item;

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Classes(pub Vec<Class>);

impl ToRedisArgs for Classes {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let serialized = serde_json::to_string(self).unwrap();
        out.write_arg(serialized.as_bytes());
    }
}

impl FromRedisValue for Classes {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let json_str: String = from_redis_value(v)?;
        Ok(serde_json::from_str::<Classes>(&json_str)?)
    }
}
