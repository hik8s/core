use std::fmt;

#[derive(Clone)]
pub enum Item {
    Fix(String), // maps to log.class.items
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
