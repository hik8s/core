use shared::{
    process::log::{compare_loglines, process_logline},
    types::record::log::LogRecord,
};
use std::env::var;
use thiserror::Error;

use crate::types::class::Class;

#[derive(Debug, Clone)]
pub struct Classifier {
    threshold: f32,
}

#[derive(Error, Debug)]
pub enum ClassificationError {
    #[error("Failed to process log line")]
    ProcessLogLineError,
    #[error("Comparison error")]
    ComparisonError,
}

impl Classifier {
    // The `new` function creates a new instance of `Classifier` with an empty vector of `ClassContext` instances,
    // a provided threshold, and `None` as the metadata.
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

    pub fn classify(
        &self,
        line: &LogRecord,
        mut classes: Vec<Class>,
    ) -> Result<String, ClassificationError> {
        // this can fail and we should probably allow it to fail
        let items = process_logline(&line.message);

        let mut best_match: Option<Class> = None;
        let mut highest_similarity = 0.0;

        for class in classes.iter_mut() {
            if class.length != items.len() {
                continue;
            }
            let aggrement = compare_loglines(items.clone(), class.mask_items());

            let similarity =
                aggrement.iter().filter(|&b| *b).count() as f32 / aggrement.len() as f32;
            if similarity > highest_similarity {
                highest_similarity = similarity;
                best_match = Some(class.clone());
            }
        }

        if let Some(mut class) = best_match {
            if highest_similarity >= self.threshold {
                class.count += 1;
                class.update_items(&items);
                return Ok(class.id);
            }
        }
        // add new class
        let class = Class::new(items);
        classes.push(class.to_owned());
        return Ok(class.id);
    }
}
