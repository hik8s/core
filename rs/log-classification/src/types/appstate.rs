#[derive(Clone)]
pub struct AppContext {
    pub app: String,
}

impl AppContext {
    pub fn new(key: &String) -> Self {
        Self {
            app: key.to_owned(),
        }
    }
}
