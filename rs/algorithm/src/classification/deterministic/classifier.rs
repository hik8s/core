use shared::{
    connections::redis::connect::{RedisConnection, RedisConnectionError},
    preprocessing::compare::compare,
    types::{classification::class::Class, record::preprocessed::PreprocessedLogRecord},
};
use std::env::var;
use thiserror::Error;

// #[derive(Clone)]
pub struct Classifier<'a> {
    threshold: f64,
    redis: &'a mut RedisConnection,
}

#[derive(Error, Debug)]
pub enum ClassificationError {
    #[error("Comparison error")]
    ComparisonError,
    #[error("Redis connection error: {0}")]
    RedisConnectionError(#[from] RedisConnectionError),
}

impl<'a> Classifier<'a> {
    // The `new` function creates a new instance of `Classifier` with an empty vector of `ClassContext` instances,
    // a provided threshold, and `None` as the metadata.
    pub fn new(threshold: Option<f64>, redis: &'a mut RedisConnection) -> Classifier<'a> {
        match threshold {
            Some(threshold) => Classifier { threshold, redis },
            None => {
                let threshold_str = match var("CLASSIFIER_THRESHOLD") {
                    Ok(val) => val,
                    Err(_) => {
                        tracing::warn!("CLASSIFIER_THRESHOLD environment variable not found, using default value of 0.7");
                        "0.7".to_string()
                    }
                };
                let threshold = threshold_str.parse::<f64>().unwrap_or_else(|_| {
                    tracing::warn!("Failed to parse CLASSIFIER_THRESHOLD as a float, using default value of 0.7");
                    0.7
                });

                Classifier { threshold, redis }
            }
        }
    }

    pub fn classify(
        &mut self,
        log: &PreprocessedLogRecord,
        key: &str,
    ) -> Result<Class, ClassificationError> {
        let mut best_match: Option<&mut Class> = None;
        let mut highest_similarity = 0 as f64;
        let state = &mut self.redis.get(&key)?;

        for class in state.classes.iter_mut() {
            if class.length != log.length as usize {
                continue;
            }
            let aggrement = compare(&log.preprocessed_message, &class.mask_items());

            let similarity =
                aggrement.iter().filter(|&b| *b).count() as f64 / aggrement.len() as f64;
            if similarity > highest_similarity {
                highest_similarity = similarity;
                best_match = Some(class);
            }
        }
        let result = match best_match {
            Some(class) => {
                if highest_similarity >= self.threshold {
                    class.count += 1;
                    class.update_items(log);
                    class.similarity = highest_similarity;
                    class.to_owned()
                } else {
                    let class = Class::from(log);
                    state.classes.push(class.to_owned());
                    class
                }
            }
            None => {
                let class = Class::from(log);
                state.classes.push(class.to_owned());
                class
            }
        };
        self.redis.set(&key, state.to_owned())?;
        Ok(result.to_owned())
    }
}

#[cfg(test)]
mod tests {

    use super::Classifier;

    use rstest::rstest;
    use shared::{
        connections::redis::connect::RedisConnection,
        tracing::setup::setup_tracing,
        types::record::preprocessed::PreprocessedLogRecord,
        utils::mock::mock_data::{get_test_data, TestCase, TestData},
    };

    #[rstest]
    #[tokio::test]
    #[case(get_test_data(TestCase::Simple))]
    async fn test_classify_json_nested(#[case] test_data: TestData) {
        setup_tracing();

        let mut redis = RedisConnection::new().unwrap();
        let mut classifier = Classifier::new(Some(0.6), &mut redis);

        for (index, (key, raw_message, expected_class)) in test_data.into_iter().enumerate() {
            let preprocessed_log = PreprocessedLogRecord::from(raw_message);
            let class = classifier.classify(&preprocessed_log, &key).unwrap();
            if index > 0 {
                assert_eq!(class.to_string(), expected_class.to_string());
                assert_eq!(class.similarity, expected_class.similarity);
            }
        }
    }
}
