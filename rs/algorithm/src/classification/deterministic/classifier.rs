use shared::{
    connections::{error::ConfigError, redis::connect::RedisConnection},
    preprocessing::compare::compare,
    types::{
        classification::{class::Class, state::ClassifierState},
        error::classificationerror::ClassificationError,
        record::{classified::ClassifiedLogRecord, preprocessed::PreprocessedLogRecord},
    },
};
use std::env::var;

const DEFAULT_THRESHOLD: f64 = 0.6;

// #[derive(Clone)]
pub struct Classifier {
    threshold: f64,
    redis: RedisConnection,
}

impl Classifier {
    // The `new` function creates a new instance of `Classifier` with an empty vector of `ClassContext` instances,
    // a provided threshold, and `None` as the metadata.
    pub fn new(threshold: Option<f64>, redis: RedisConnection) -> Result<Classifier, ConfigError> {
        let threshold = threshold.unwrap_or(
            var("CLASSIFIER_THRESHOLD")
                .unwrap_or(DEFAULT_THRESHOLD.to_string())
                .parse::<f64>()
                .map_err(ConfigError::ParseFloatError)?,
        );
        Ok(Classifier { threshold, redis })
    }

    pub fn classify(
        &mut self,
        log: &PreprocessedLogRecord,
        key: &String,
    ) -> Result<(Option<Class>, ClassifiedLogRecord), ClassificationError> {
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
                    let previous_class = class.clone();
                    class.count += 1;
                    class.update_items(log);
                    class.similarity = highest_similarity;
                    // TODO: return class if representation changed
                    let identical = previous_class.to_string() == class.to_string();
                    let classified_log = ClassifiedLogRecord::new(log, &class);
                    (self.get_class(class, identical), classified_log)
                } else {
                    self.new_class(log, state, key)
                }
            }
            None => self.new_class(log, state, key),
        };
        self.redis.set(&key, state.to_owned())?;
        Ok(result)
    }

    fn new_class(
        &self,
        log: &PreprocessedLogRecord,
        state: &mut ClassifierState,
        key: &String,
    ) -> (Option<Class>, ClassifiedLogRecord) {
        let class = Class::from_log_and_key(log, key);
        state.classes.push(class.to_owned());
        // always return new class
        (Some(class.clone()), ClassifiedLogRecord::new(log, &class))
    }
    fn get_class(&self, class: &mut Class, return_none: bool) -> Option<Class> {
        match return_none {
            true => None,
            false => Some(class.clone()),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::Classifier;
    use rstest::rstest;
    use shared::{
        connections::redis::connect::RedisConnection,
        tracing::setup::setup_tracing,
        types::{error::testerror::TestError, record::preprocessed::PreprocessedLogRecord},
        utils::mock::mock_data::{get_test_data, TestCase, TestData},
    };

    #[rstest]
    #[tokio::test]
    #[case(get_test_data(TestCase::Simple))]
    async fn test_classify_json_nested(#[case] test_data: TestData) -> Result<(), TestError> {
        setup_tracing();

        let redis = RedisConnection::new()?;
        let mut classifier = Classifier::new(Some(0.6), redis)?;

        for (index, (key, raw_message, expected_class)) in test_data.into_iter().enumerate() {
            let preprocessed_log = PreprocessedLogRecord::from(raw_message);
            let class = classifier.classify(&preprocessed_log, &key)?.0;
            if index == 1 {
                assert_eq!(class.is_none(), false);
                let class = class.unwrap();
                assert_eq!(class.to_string(), expected_class.to_string());
                assert_eq!(class.similarity, expected_class.similarity);
            } else if index > 1 {
                assert_eq!(class.is_none(), true);
            }
        }
        Ok(())
    }
}
