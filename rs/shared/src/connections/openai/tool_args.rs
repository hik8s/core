use serde::{Deserialize, Serialize};

use crate::testdata::UserTestData;

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
        let mut prompt: String = user_message.to_string();
        prompt.push_str(format!("\nUser intention: {}", self.intention).as_str());

        if let Some(kind) = &self.kind {
            prompt.push_str(&format!("\nKind: {}", kind));
        }
        if let Some(name) = &self.name {
            prompt.push_str(&format!("\nName: {}", name));
        }
        if let Some(namespace) = &self.namespace {
            prompt.push_str(&format!("\nNamespace: {}", namespace));
        }
        prompt
    }
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

impl EventRetrievalArgs {
    pub fn search_prompt(&self, user_message: &str) -> String {
        let mut prompt: String = user_message.to_string();
        prompt.push_str(format!("\nUser intention: {}", self.intention).as_str());

        if self.application.is_some() {
            prompt.push_str(&format!(
                "\nApplication: {}",
                self.application.as_ref().unwrap()
            ));
        }
        if self.namespace.is_some() {
            prompt.push_str(&format!(
                "\nNamespace: {}",
                self.namespace.as_ref().unwrap()
            ));
        }
        if self.kind.is_some() {
            prompt.push_str(&format!("\nKind: {}", self.kind.as_ref().unwrap()));
        }
        prompt
    }
}
