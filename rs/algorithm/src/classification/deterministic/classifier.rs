use shared::{
    preprocessing::compare::compare,
    types::{
        class::Class,
        classifier::{error::ClassifierError, state::ClassifierState},
        record::{classified::ClassifiedLogRecord, preprocessed::PreprocessedLogRecord},
        tokenizer::Tokenizer,
    },
    DbName, RedisConnection,
};
use std::env::var;

const DEFAULT_THRESHOLD: f64 = 0.6;

// #[derive(Clone)]
pub struct Classifier {
    threshold: f64,
    redis: RedisConnection,
    tokenizer: Tokenizer,
}

impl Classifier {
    // The `new` function creates a new instance of `Classifier` with an empty vector of `ClassContext` instances,
    // a provided threshold, and `None` as the metadata.
    pub fn new(
        threshold: Option<f64>,
        redis: RedisConnection,
    ) -> Result<Classifier, ClassifierError> {
        let threshold = threshold.unwrap_or(
            var("CLASSIFIER_THRESHOLD")
                .unwrap_or(DEFAULT_THRESHOLD.to_string())
                .parse::<f64>()?,
        );
        let tokenizer = Tokenizer::new()?;
        Ok(Classifier {
            threshold,
            redis,
            tokenizer,
        })
    }

    pub fn classify(
        &mut self,
        log: &PreprocessedLogRecord,
        customer_id: &str,
    ) -> Result<(Option<Class>, ClassifiedLogRecord), ClassifierError> {
        let mut best_match: Option<&mut Class> = None;
        let mut highest_similarity = 0 as f64;
        let key = self.redis.key(DbName::Log, customer_id, None, &log.key);
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
                    let classified_log = ClassifiedLogRecord::new(log, class);
                    (self.get_class(class, identical), classified_log)
                } else {
                    self.new_class(log, state)
                }
            }
            None => self.new_class(log, state),
        };
        self.redis.set(&key, state.to_owned())?;
        Ok(result)
    }

    fn new_class(
        &self,
        log: &PreprocessedLogRecord,
        state: &mut ClassifierState,
    ) -> (Option<Class>, ClassifiedLogRecord) {
        let mut class = Class::from_log_and_token_count(log, 0);
        class.token_count = self.token_count(&class);
        state.classes.push(class.to_owned());
        // always return new class
        (Some(class.clone()), ClassifiedLogRecord::new(log, &class))
    }
    fn get_class(&self, class: &mut Class, return_none: bool) -> Option<Class> {
        match return_none {
            true => None,
            false => {
                class.token_count = self.token_count(class);
                Some(class.clone())
            }
        }
    }

    fn token_count(&self, class: &Class) -> u32 {
        self.tokenizer
            .calculate_token_length(class.to_string().as_str()) as u32
    }
}

#[cfg(test)]
mod tests {

    use super::Classifier;
    use rstest::rstest;
    use shared::{
        tracing::setup::setup_tracing,
        types::{classifier::error::ClassifierError, record::preprocessed::PreprocessedLogRecord},
        utils::mock::mock_data::{get_test_data, TestCase, TestData},
        RedisConnection,
    };

    #[tokio::test]
    #[rstest]
    #[case(get_test_data(TestCase::Simple))]
    #[case(get_test_data(TestCase::Short))]
    async fn test_classify_json_nested(#[case] test_data: TestData) -> Result<(), ClassifierError> {
        setup_tracing(false);

        let redis = RedisConnection::new()?;
        let mut classifier = Classifier::new(Some(0.6), redis)?;

        for (index, (key, raw_message, expected_class)) in test_data.into_iter().enumerate() {
            let preprocessed_log =
                PreprocessedLogRecord::from((&"customer_id".to_owned(), &raw_message, &key));
            let class = classifier.classify(&preprocessed_log, "customer_id")?.0;
            if let Some(class) = &class {
                tracing::debug!("Classified: {}", class);
            }

            match index.cmp(&1) {
                std::cmp::Ordering::Equal => {
                    assert!(class.is_some());
                    let class = class.unwrap();
                    assert_eq!(class.to_string(), expected_class.to_string());
                    assert_eq!(class.similarity, expected_class.similarity);
                }
                std::cmp::Ordering::Greater => {
                    assert!(class.is_none());
                }
                std::cmp::Ordering::Less => {}
            }
        }
        Ok(())
    }
}
