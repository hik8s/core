use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Item {
    Fix(String),
    Var,
}
impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Fix(s) => write!(f, "{}", s),
            Item::Var => write!(f, "<var>"),
        }
    }
}
