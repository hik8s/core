use std::fmt::{Display, Formatter, Result};

use crate::{
    types::{class::Class, metadata::Metadata, record::preprocessed::PreprocessedLogRecord},
    utils::mock::mock_client::{generate_podname, get_test_metadata},
};
use async_openai::types::ChatCompletionRequestMessage;
use strum::{EnumIter, EnumString, IntoEnumIterator};

#[derive(Debug, EnumIter, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum UserTest {
    Logs,
    RetrieveLogs,
    RetrieveLogsForMe,
    RetrieveLogsForClusterForMe,
    RetrieveLogsAppNamespace,
    LogsAppNamespace,
    LogsAppNamespaceForMe,
    PodKillOutOffMemory,
}

impl UserTest {
    fn prompt(&self, meta: &Metadata) -> String {
        match self {
            UserTest::Logs => format!("logs"),
            UserTest::RetrieveLogs => format!("Could you retrieve logs?"),
            UserTest::RetrieveLogsForMe => format!("Could you retrieve logs for me?"),
            UserTest::RetrieveLogsForClusterForMe => format!("Could you retrieve logs for the cluster for me?"),
            UserTest::RetrieveLogsAppNamespace => format!("Could you investigate the logs from {} in {}?", &meta.pod_name, &meta.namespace),
            UserTest::LogsAppNamespace => format!("{} logs in {}?", &meta.pod_name, &meta.namespace),
            UserTest::LogsAppNamespaceForMe => format!("{} logs in {} for me?", &meta.pod_name, &meta.namespace),
            UserTest::PodKillOutOffMemory => format!("I have a problem with my application called {} in namespace {}? Could you investigate the logs and also provide an overview of the cluster?", &meta.pod_name, &meta.namespace),
        }
    }
    fn log_message(&self) -> String {
        match self {
            UserTest::PodKillOutOffMemory => "OOMKilled Exit Code 137".to_owned(),
            _ => "".to_owned(),
        }
    }
}

pub struct UserTestData {
    pub prompt: String,
    pub class: Class,
    pub messages: Vec<ChatCompletionRequestMessage>,
    pub application: Option<String>,
    pub namespace: Option<String>,
}

impl Display for UserTest {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let variant_name = format!("{:?}", self)
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if i > 0 && c.is_uppercase() {
                    vec!['-', c.to_ascii_lowercase()]
                } else {
                    vec![c.to_ascii_lowercase()]
                }
            })
            .collect::<String>();
        write!(f, "userdata-{}", variant_name)
    }
}
impl UserTestData {
    pub fn new(case: UserTest) -> Self {
        let mut meta = get_test_metadata(&generate_podname(&case));
        let app = Some("logd".to_owned());
        let ns = Some("hik8s-system".to_owned());
        match case {
            UserTest::PodKillOutOffMemory => UserTestData {
                prompt: case.prompt(&meta),
                class: log_class(&case.log_message(), &meta),
                messages: vec![
                    ChatCompletionRequestMessage::System(Default::default()),
                    ChatCompletionRequestMessage::User(Default::default()),
                    ChatCompletionRequestMessage::Assistant(Default::default()),
                    ChatCompletionRequestMessage::Tool(Default::default()),
                    ChatCompletionRequestMessage::Tool(Default::default()),
                ],
                application: Some(meta.pod_name.to_owned()),
                namespace: Some(meta.namespace.to_owned()),
            },
            UserTest::RetrieveLogsAppNamespace => UserTestData::from_case(case, &mut meta, app, ns),
            UserTest::LogsAppNamespace => UserTestData::from_case(case, &mut meta, app, ns),
            UserTest::LogsAppNamespaceForMe => UserTestData::from_case(case, &mut meta, app, ns),
            _ => UserTestData::from_case(case, &mut meta, None, None),
        }
    }

    // pub fn get_corpus(&self) -> Vec<String> {
    //     vec![self.to_string()]
    // }
}

impl UserTestData {
    fn from_case(
        case: UserTest,
        meta: &mut Metadata,
        application: Option<String>,
        namespace: Option<String>,
    ) -> Self {
        if let Some(app) = &application {
            meta.pod_name = app.to_owned();
        }
        if let Some(ns) = &namespace {
            meta.namespace = ns.to_owned();
        }
        Self {
            prompt: case.prompt(&meta),
            class: log_class(&case.log_message(), &meta),
            messages: vec![],
            application,
            namespace,
        }
    }
}

fn log_class(text: &String, meta: &Metadata) -> Class {
    let customer_id = "test_id_123".to_owned();
    let log = PreprocessedLogRecord::from((&customer_id, text, meta));
    Class::new(&log, 0)
}

impl UserTest {
    pub fn corpus() -> Vec<String> {
        let base = "I've retrieved the logs for the application in the namespace, it appears";
        let mut corpus: Vec<String> = UserTest::iter()
            .filter(|case| !case.log_message().is_empty())
            .map(|case| format!("{base} {}", case.log_message()))
            .collect();
        corpus.push(format!("{base} that there is not problem found."));
        corpus.push(format!("{base} that the logs are empty."));
        corpus
    }
}
