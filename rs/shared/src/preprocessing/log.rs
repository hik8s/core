use serde_json::Value;

use crate::types::record::log::LogRecord;

pub fn preprocess_message(log: &LogRecord) -> Vec<String> {
    let message = log.message.as_str();
    let mut result = Vec::new();

    // Check if the input string contains a JSON object
    if let (Some(start_index), Some(end_index)) = (message.find('{'), message.rfind('}')) {
        // Process the part of the input string before the JSON object
        result.extend(split_string(&message[0..start_index]));

        // Extract the JSON object from the string
        let json_str = &message[start_index..=end_index];

        // Parse the JSON object
        match serde_json::from_str::<Value>(json_str) {
            Ok(json) => {
                result.extend(flatten_json(&json));
            }
            Err(e) => {
                tracing::debug!(
                    "Could not parse as json, continue with split. Error: {}, {}, {}",
                    e,
                    log.key,
                    log.record_id
                );
                result.extend(split_string(json_str));
            }
        };

        // Process the part of the input string after the JSON object
        result.extend(split_string(&message[end_index + 1..]));
    } else {
        // If the input string does not contain a JSON object, process it as a non-JSON string
        result.extend(split_string(&message));
    }
    result
}

fn split_string(input: &str) -> Vec<String> {
    input.split_whitespace().map(|s| s.to_string()).collect()
}

fn flatten_json(json: &Value) -> Vec<String> {
    let mut result = Vec::new();
    flatten_json_recursive(json, &mut result, String::new());
    result
}

fn flatten_json_recursive(json: &Value, result: &mut Vec<String>, prefix: String) {
    match json {
        Value::Object(map) => {
            for (key, value) in map {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                result.push(new_prefix.clone());
                flatten_json_recursive(value, result, new_prefix);
            }
        }
        Value::Array(arr) => {
            for (index, value) in arr.iter().enumerate() {
                let new_prefix = format!("{}[{}]", prefix, index);
                flatten_json_recursive(value, result, new_prefix);
            }
        }
        Value::String(s) => {
            let cleaned_string = s.replace("\\\"", "\"").replace("\\'", "'");
            result.push(cleaned_string);
        }
        _ => {
            result.push(json.to_string());
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        preprocessing::compare::compare, tracing::setup::setup_tracing,
        types::record::log::get_test_log_record,
    };
    use rstest::rstest;
    use serde_json::json;

    #[rstest]
    #[case("stderr F {\"level\":\"info\",\"ts\":\"2024-03-16T05:28:18.752849Z\",\"caller\":\"mvcc/hash.go:137\",\"msg\":\"storing new hash\",\"hash\":3811437805,\"revision\":108791,\"compact-revision\":108342}",
        vec![
            "stderr",
            "F",
            "level", "info",
            "ts", "2024-03-16T05:28:18.752849Z",
            "caller", "mvcc/hash.go:137",
            "msg", "storing new hash",
            "hash", "3811437805",
            "revision", "108791",
            "compact-revision", "108342",
        ]
    )]
    #[case("stderr F I0315 09:37:55.934101       1 main.go:250] Node kind-worker2 has CIDR [10.244.2.0/24]", vec!["stderr", "F", "I0315", "09:37:55.934101", "1", "main.go:250]", "Node", "kind-worker2", "has", "CIDR", "[10.244.2.0/24]"])]
    #[case("stderr F I0315 10:44:54.473228       1 main.go:227] handling current node", vec!["stderr", "F", "I0315", "10:44:54.473228", "1", "main.go:227]", "handling", "current", "node"])]
    fn test_preprocess_log(#[case] input: &str, #[case] expected: Vec<&str>) {
        setup_tracing();
        assert_eq!(preprocess_message(&get_test_log_record(input)), expected);
    }

    #[rstest]
    #[case(json!({
        "name": "John",
        "age": 30,
        "cars": [
            {"model": "Ford", "mpg": 10.5},
            {"model": "BMW", "mpg": 15.2}
        ]
    }), vec![
        "name", "John",
        "age", "30",
        "cars",
        "cars[0].model", "Ford",
        "cars[0].mpg", "10.5",
        "cars[1].model", "BMW",
        "cars[1].mpg", "15.2",
    ])]
    #[case(json!({
        "level":"info",
        "ts":"2024-03-16T05:28:18.752849Z",
        "caller":"mvcc/hash.go:137",
        "msg":"storing new hash",
        "hash":2147483647,
        "revision":108791,
        "compact-revision":108342
    }), vec![
        "level", "info",
        "ts", "2024-03-16T05:28:18.752849Z",
        "caller", "mvcc/hash.go:137",
        "msg", "storing new hash",
        "hash", "2147483647",
        "revision", "108791",
        "compact-revision", "108342",
    ])]
    fn test_flatten_json(#[case] json: Value, #[case] expected: Vec<&str>) {
        setup_tracing();
        assert_eq!(flatten_json(&json), expected);
    }

    #[rstest]
    #[case((
        "stderr F {\"level\":\"info\",\"ts\":\"2024-03-15T13:30:42.353083Z\",\"caller\":\"mvcc/hash.go:137\",\"msg\":\"storing new hash\",\"hash\":3759471452,\"revision\":77344,\"compact-revision\":76896}", 
        "stderr F {\"level\":\"info\",\"ts\":\"2024-03-15T13:10:41.454817Z\",\"caller\":\"mvcc/hash.go:137\",\"msg\":\"storing new hash\",\"hash\":3660114837,\"revision\":75997,\"compact-revision\":75548}"), 
        vec![true, true, true, true, true, false, true, true, true, true, true, false, true, false, true, false]
    )]
    #[case((
        "stderr F I0315 09:37:55.934101       1 main.go:250] Node kind-worker2 has CIDR [10.244.2.0/24]", 
        "stderr F I0315 10:44:54.473228       1 main.go:227] handling current node"), 
        vec![true, true, true, false, true, false, false, false, false, false, false]
    )]
    #[case((
        "stderr F I0315 09:37:55.934101       1 main.go:250] Node kind-worker2 has CIDR [10.244.2.0/24]", 
        "stderr F I0315 09:37:55.934101       1 main.go:250] Node kind-worker2 has CIDR [10.244.2.0/24]"),
        vec![true, true, true, true, true, true, true, true, true, true, true]
    )]

    fn test_preprocess_compare_logs(#[case] inputs: (&str, &str), #[case] expected: Vec<bool>) {
        setup_tracing();
        let (input1, input2) = inputs;
        let preprocessed_input1 = preprocess_message(&get_test_log_record(input1));
        let preprocessed_input2 = preprocess_message(&get_test_log_record(input2));

        let comparison = compare(&preprocessed_input1, &preprocessed_input2);
        assert_eq!(comparison, expected, "All items should match");
    }
}
