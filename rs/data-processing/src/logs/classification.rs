use shared::types::parsedline::ParsedLine;
use std::env::var;
use tracing::info;
#[derive(Debug, Clone)]
pub struct Classifier {
    threshold: f32,
}

impl Classifier {
    pub fn new(threshold: Option<f32>) -> Classifier {
        match threshold {
            Some(threshold) => Classifier { threshold },
            None => {
                let threshold_str = match var("CLASSIFIER_THRESHOLD") {
                    Ok(val) => val,
                    Err(_) => {
                        tracing::warn!("CLASSIFIER_THRESHOLD environment variable not found, using default value of 0.7");
                        "0.7".to_string()
                    }
                };
                let threshold = threshold_str.parse::<f32>().unwrap_or_else(|_| {
                    tracing::warn!("Failed to parse CLASSIFIER_THRESHOLD as a float, using default value of 0.7");
                    0.7
                });

                Classifier { threshold }
            }
        }
    }

    pub fn classify(&self, line: &ParsedLine, app: String) {
        let ParsedLine {
            timestamp,
            text,
            id,
        } = line;
        info!(
            "app: {:?} timestamp: {:?} text: {:?} id: {:?}",
            app, timestamp, text, id
        );
    }
}
