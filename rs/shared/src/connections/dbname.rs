use std::fmt;

#[derive(PartialEq)]
pub enum DbName {
    Log,
    Resource,
    CustomResource,
    Event,
}

impl fmt::Display for DbName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DbName::Log => "log",
            DbName::Resource => "resource",
            DbName::CustomResource => "customresource",
            DbName::Event => "event",
        };
        write!(f, "{}", name)
    }
}

impl DbName {
    pub fn id(&self, customer_id: &str) -> String {
        format!("{self}_{customer_id}")
    }
}
