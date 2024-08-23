use std::collections::HashMap;
use std::sync::PoisonError;
use std::sync::RwLock;

use crate::types::classification::class::Class;
use redis::ToRedisArgs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClassifierStateError {
    #[error("Poison error: {0}")]
    PoisonError(String),
}

impl<T> From<PoisonError<T>> for ClassifierStateError {
    fn from(e: PoisonError<T>) -> Self {
        ClassifierStateError::PoisonError(e.to_string())
    }
}

pub struct ClassifierState {
    pub state: RwLock<HashMap<String, Vec<Class>>>,
}

impl ClassifierState {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_or_create(&self, key: &String) -> Result<Vec<Class>, ClassifierStateError> {
        // TODO: add key to error message
        let mut state = self.state.write()?;
        match state.get(key) {
            Some(app) => Ok(app.clone()),
            None => {
                let classes = Vec::<Class>::new();
                state.insert(key.to_owned(), classes.clone());
                Ok(classes)
            }
        }
    }

    pub async fn insert(&self, key: &str, classes: Vec<Class>) -> Result<(), ClassifierStateError> {
        let mut state = self.state.write()?;
        state.insert(key.to_string(), classes);
        Ok(())
    }
}

impl ToRedisArgs for ClassifierState {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let state = self.state.read().unwrap();
        for (key, value) in state.iter() {
            out.write_arg_fmt(format!("{}: {:?}", key, value));
        }
    }
}
