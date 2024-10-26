use std::fmt::{Display, Formatter, Result};

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
            UserInputTestCase::WhatIsGoingOnWithApp => "What is going on with logd?",
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
