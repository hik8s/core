use serde::{Deserialize, Serialize};

use crate::testdata::UserTestData;

pub fn format_tool_args<T: fmt::Display>(args: &T) -> String {
    args.to_string()
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ResourceStatusRetrievalArgs {
    pub namespace: Option<String>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub intention: String,
}

impl TryFrom<String> for ResourceStatusRetrievalArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}

impl ResourceStatusRetrievalArgs {
    pub fn search_prompt(&self, user_message: &str) -> String {
        format_search_prompt(self, user_message)
    }
}
impl EventRetrievalArgs {
    pub fn search_prompt(&self, user_message: &str) -> String {
        format_search_prompt(self, user_message)
    }
}
pub fn format_search_prompt<T: fmt::Display>(args: &T, user_message: &str) -> String {
    let mut prompt = user_message.to_string();
    prompt.push('\n');
    prompt.push_str(&args.to_string());
    prompt
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ClusterOverviewArgs {
    pub resources: Vec<String>,
}

impl TryFrom<String> for ClusterOverviewArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CreateDeploymentArgs {
    pub databases: Vec<String>,
    pub name: String,
    pub namespace: String,
    pub image_name: String,
}

impl TryFrom<String> for CreateDeploymentArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct LogRetrievalArgs {
    pub namespace: Option<String>,
    pub application: Option<String>,
    pub intention: String,
}
impl LogRetrievalArgs {
    pub fn new(testdata: &UserTestData) -> Self {
        LogRetrievalArgs {
            namespace: testdata.namespace.to_owned(),
            application: testdata.application.to_owned(),
            intention: "".to_owned(),
        }
    }
}
impl TryFrom<String> for LogRetrievalArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct EventRetrievalArgs {
    pub namespace: Option<String>,
    pub application: Option<String>,
    pub kind: Option<String>,
    pub intention: String,
}

impl TryFrom<String> for EventRetrievalArgs {
    type Error = serde_json::Error;

    fn try_from(json_string: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json_string)
    }
}

use std::fmt;

impl fmt::Display for ResourceStatusRetrievalArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = [
            format!("kind={}", self.kind.as_deref().unwrap_or("None")),
            format!("name={}", self.name.as_deref().unwrap_or("None")),
            format!("namespace={}", self.namespace.as_deref().unwrap_or("None")),
            format!("intention=\"{}\"", self.intention),
        ];

        write!(f, "{}", parts.join(", "))
    }
}

impl fmt::Display for ClusterOverviewArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resources=[{}]", self.resources.join(", "))
    }
}

impl fmt::Display for CreateDeploymentArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = [
            format!("name={}", self.name),
            format!("namespace={}", self.namespace),
            format!("image={}", self.image_name),
            format!("databases=[{}]", self.databases.join(", ")),
        ];

        write!(f, "{}", parts.join(", "))
    }
}

impl fmt::Display for LogRetrievalArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = [
            format!("namespace={}", self.namespace.as_deref().unwrap_or("None")),
            format!(
                "application={}",
                self.application.as_deref().unwrap_or("None")
            ),
            format!("intention=\"{}\"", self.intention),
        ];

        write!(f, "{}", parts.join(", "))
    }
}

impl fmt::Display for EventRetrievalArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = [
            format!("namespace={}", self.namespace.as_deref().unwrap_or("None")),
            format!(
                "application={}",
                self.application.as_deref().unwrap_or("None")
            ),
            format!("kind={}", self.kind.as_deref().unwrap_or("None")),
            format!("intention=\"{}\"", self.intention),
        ];

        write!(f, "{}", parts.join(", "))
    }
}
