use std::collections::HashMap;
use std::sync::PoisonError;
use std::sync::RwLock;

use super::appcontext::AppContext;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppStateError {
    #[error("Poison error: {0}")]
    PoisonError(String),
    #[error("App context creation error: {0}")]
    AppContextCreationError(String),
}

impl<T> From<PoisonError<T>> for AppStateError {
    fn from(e: PoisonError<T>) -> Self {
        AppStateError::PoisonError(e.to_string())
    }
}

pub struct AppState {
    apps: RwLock<HashMap<String, AppContext>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            apps: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_or_create(&self, key: &String) -> Result<AppContext, AppStateError> {
        let mut apps = self.apps.write()?;
        match apps.get(key) {
            Some(app) => Ok(app.clone()),
            None => {
                let app = AppContext::new(key);
                apps.insert(key.to_owned(), app.clone());
                Ok(app)
            }
        }
    }

    pub async fn insert(&self, pod_name: &str, app: AppContext) -> Result<(), AppStateError> {
        let mut apps = self.apps.write()?;
        apps.insert(pod_name.to_string(), app);
        Ok(())
    }
}
