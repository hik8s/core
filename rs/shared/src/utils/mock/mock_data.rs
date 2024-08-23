use std::fmt;

use rand::distributions::{Distribution, Uniform};

#[derive(Clone, Copy)]
pub enum TestCase {
    Simple,
}

impl fmt::Display for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            TestCase::Simple => "simple",
        };
        write!(f, "{}", name)
    }
}
