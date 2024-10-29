use std::fmt::{Display, Formatter, Result};

use shared::{
    types::class::{item::Item, Class},
    utils::mock::{
        mock_client::{generate_podname, get_test_metadata},
        mock_data::class_from_items,
    },
};

use super::understanding::UnderstandingOutput;

// #[derive(Debug)]
pub enum UserInputTestCase {
    WhatIsGoingOn,
    WhatIsGoingOnWithApp,
    WhatIsGoingOnWithNamespace,
    WhatIsGoingOnWithAppWithNamespace,
}

impl Display for UserInputTestCase {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let name = match self {
            UserInputTestCase::WhatIsGoingOn => "What is going on?",
            UserInputTestCase::WhatIsGoingOnWithApp => "What is going on with application logd?",
            UserInputTestCase::WhatIsGoingOnWithNamespace => {
                "What is going on with namespace hik8s-stag?"
            }
            UserInputTestCase::WhatIsGoingOnWithAppWithNamespace => {
                "What is going on with logd in hik8s-stag?"
            }
        };
        write!(f, "test-{}", name)
    }
}

pub fn get_expected_output(test_case: UserInputTestCase) -> UnderstandingOutput {
    match test_case {
        UserInputTestCase::WhatIsGoingOn => UnderstandingOutput {
            application: None,
            namespace: None,
            keywords: vec!["What is going on".to_string()],
            intention: Some("What is going on".to_string()),
        },
        UserInputTestCase::WhatIsGoingOnWithApp => UnderstandingOutput {
            application: Some("logd".to_string()),
            namespace: None,
            keywords: vec!["What is going on".to_string()],
            intention: Some("What is going on".to_string()),
        },
        UserInputTestCase::WhatIsGoingOnWithNamespace => UnderstandingOutput {
            application: None,
            namespace: Some("hik8s-stag".to_string()),
            keywords: vec!["What is going on".to_string()],
            intention: Some("What is going on".to_string()),
        },
        UserInputTestCase::WhatIsGoingOnWithAppWithNamespace => UnderstandingOutput {
            application: Some("logd".to_string()),
            namespace: Some("hik8s-stag".to_string()),
            keywords: vec!["What is going on".to_string()],
            intention: Some("What is going on".to_string()),
        },
    }
}

pub enum ClusterTestScenario {
    PodKillOutOffMemory,
}
impl Display for ClusterTestScenario {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let name = match self {
            ClusterTestScenario::PodKillOutOffMemory => "oom-kill",
        };
        write!(f, "scenario-{}", name)
    }
}

pub struct TestScenarioData {
    pub vectorized_classes: Vec<Class>,
    pub expected_final_solution: String,
}

pub fn get_scenario_data(test_scenario: &ClusterTestScenario) -> TestScenarioData {
    let metadata = get_test_metadata(&generate_podname(test_scenario));

    match test_scenario {
        ClusterTestScenario::PodKillOutOffMemory => {
            let vectorized_classes = vec![class_from_items(
                vec![
                    Item::Fix("OOMKilled".to_string()),
                    Item::Fix("Exit".to_string()),
                    Item::Fix("Code".to_string()),
                    Item::Fix("137".to_string()),
                ],
                0.875,
                &metadata,
            )];
            let expected_final_solution = "The pod was killed out of memory and you should increase the memory limit to a reasonalbe value".to_string();
            TestScenarioData {
                vectorized_classes,
                expected_final_solution,
            }
        }
    }
}
