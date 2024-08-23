use std::collections::HashMap;
use std::sync::PoisonError;

use crate::types::classification::class::Class;
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
    pub state: HashMap<String, Vec<Class>>,
}

impl ClassifierState {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    pub async fn get_or_create(
        &mut self,
        key: &String,
    ) -> Result<Vec<Class>, ClassifierStateError> {
        // TODO: add key to error message
        match self.state.get(key) {
            Some(app) => Ok(app.clone()),
            None => {
                let classes = Vec::<Class>::new();
                self.state.insert(key.to_owned(), classes.clone());
                Ok(classes)
            }
        }
    }

    pub async fn insert(
        &mut self,
        key: &str,
        classes: Vec<Class>,
    ) -> Result<(), ClassifierStateError> {
        self.state.insert(key.to_string(), classes);
        Ok(())
    }
}
