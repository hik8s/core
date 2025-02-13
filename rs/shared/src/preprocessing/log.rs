use serde_json::Value;

pub fn preprocess_message(
    message: &str,
    customer_id: &str,
    key: &str,
    record_id: &str,
) -> Vec<String> {
    let mut result = Vec::new();

    // Check if the input string contains a JSON object
    if let (Some(start_index), Some(end_index)) = (message.find('{'), message.rfind('}')) {
        // Validate indices before slicing
        if start_index >= end_index {
            tracing::warn!("Invalid JSON bounds. Skipping");
            return split_string(message);
        }

        // Process the part before JSON
        result.extend(split_string(&message[0..start_index]));

        // Extract and parse JSON
        let json_str = &message[start_index..=end_index];
        match serde_json::from_str::<Value>(json_str) {
            Ok(json) => {
                result.extend(flatten_json(&json));
            }
            Err(e) => {
                tracing::debug!(
                    "Could not parse as json, continue with split. Error: {}, id: {}, key: {}, record: {}",
                    e,
                    customer_id,
                    key,
                    record_id
                );
                result.extend(split_string(json_str));
            }
        };

        // Process the part of the input string after the JSON object
        result.extend(split_string(&message[end_index + 1..]));
    } else {
        // If the input string does not contain a JSON object, process it as a non-JSON string
        result.extend(split_string(message));
    }
    result
}

fn split_string(input: &str) -> Vec<String> {
    input
        .replace("[", " [ ")
        .replace("]", " ] ")
        .split_whitespace()
        .flat_map(|s| {
            let comma_parts: Vec<&str> = s.split(',').collect();
            comma_parts.into_iter().flat_map(|part| {
                // Find first colon
                if let Some(colon_index) = part.find(':') {
                    let (left, right) = part.split_at(colon_index);
                    // Check if right part exists (remove the colon)
                    let right = &right[1..];

                    // Check if left is not numeric and both parts are non-empty
                    if !left.is_empty()
                        && !right.is_empty()
                        && !left.chars().next().map_or(false, |c| c.is_numeric())
                        && !right.chars().all(|c| c.is_numeric())
                    {
                        vec![left.to_string(), right.to_string()]
                    } else {
                        vec![part.to_string()]
                    }
                } else {
                    vec![part.to_string()]
                }
            })
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
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

    use std::iter::zip;

    use super::*;
    use crate::{preprocessing::compare::compare, tracing::setup::setup_tracing};
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
        setup_tracing(false);
        assert_eq!(
            preprocess_message(input, "customer_id", "key", "record_id"),
            expected
        );
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
        setup_tracing(false);
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
        vec![true, true, true, false, true, false, true, false, false, false, false, false, false, false]
    )]
    #[case((
        "stderr F I0315 09:37:55.934101       1 main.go:250] Node kind-worker2 has CIDR [10.244.2.0/24]", 
        "stderr F I0315 09:37:55.934101       1 main.go:250] Node kind-worker2 has CIDR [10.244.2.0/24]"),
        vec![true, true, true, true, true, true, true, true, true, true, true, true, true, true]
    )]
    #[case((
        "Trace[535324451]: [878.754588ms] [878.754588ms] END", 
        "Trace[803048940]: [1h59m31.217943757s] [1h59m31.217943757s] END"),
        vec![true, true, false, true, true, true, false, true, true, false, true, true]
    )]
    #[case((
        r#"Trace[535324451]:  ---"Txn call completed" 877ms (20:22:51.902)"#, 
        r#"Trace[631464459]:  ---"Txn call completed" 733ms (20:22:51.902)"#),
        vec![true, true, false, true, true, true, true, true, false, true, true]
    )]
    #[case((
        r#"Trace[535324451]: ["GuaranteedUpdate etcd3" audit-id:9fde19f1-4dd0-45f3-acc3-dc5d15a6e61d,key:/leases/kube-system/apiserver-347x4lfleh74trmmrparvvqcqq,type:*coordination.Lease,resource:leases.coordination.k8s.io 878ms (20:22:51.023)"#, 
        r#"Trace[631464459]: ["GuaranteedUpdate etcd3" audit-id:27328d45-5870-4189-aff5-f674130f5480,key:/leases/kube-system/apiserver-347x4lfleh74trmmrparvvqcqq,type:*coordination.Lease,resource:leases.coordination.k8s.io 737ms (20:22:51.165)"#),
        vec![true, true, false, true, true, true, true, true, true, false, true, true, true, true, true, true, false, true, false]
    )]
    #[case((
        r#"I0212 20:22:51.902250       1 trace.go:236] Trace[535324451]: "Update" accept:application/vnd.kubernetes.protobuf, */*,audit-id:9fde19f1-4dd0-45f3-acc3-dc5d15a6e61d,client:::1,api-group:coordination.k8s.io,api-version:v1,name:apiserver-347x4lfleh74trmmrparvvqcqq,subresource:,namespace:kube-system,protocol:HTTP/2.0,resource:leases,scope:resource,url:/apis/coordination.k8s.io/v1/namespaces/kube-system/leases/apiserver-347x4lfleh74trmmrparvvqcqq,user-agent:kube-apiserver/v1.30.1 (linux/amd64) kubernetes/6911225,verb:PUT (12-Feb-2025 20:22:51.023) (total time: 878ms):"#, 
        r#"I0212 20:22:51.644891       1 trace.go:236] Trace[1589317137]: "Update" accept:application/json, */*,audit-id:a8ccd550-7e16-40d2-9679-f9d80a6c57bd,client:10.244.1.86,api-group:coordination.k8s.io,api-version:v1,name:notification-controller-leader-election,subresource:,namespace:flux-system,protocol:HTTP/2.0,resource:leases,scope:resource,url:/apis/coordination.k8s.io/v1/namespaces/flux-system/leases/notification-controller-leader-election,user-agent:notification-controller/v0.0.0 (linux/amd64) kubernetes/$Format/leader-election,verb:PUT (12-Feb-2025 20:22:50.978) (total time: 666ms):"#
    ),
        vec![true, false, true, true, true, true, true, false, true, true, true, true, false, true, true, false, true, false, true, true, true, true, true, false, true, true, false, true, true, true, true, true, true, true, false, true, false, true, false, true, true, true, false, true, true, false]
    )]

    fn test_preprocess_compare_logs(#[case] inputs: (&str, &str), #[case] expected: Vec<bool>) {
        setup_tracing(false);
        let (input1, input2) = inputs;
        let preprocessed_input1 = preprocess_message(input1, "customer_id", "key", "record_id1");
        let preprocessed_input2 = preprocess_message(input2, "customer_id", "key", "record_id2");
        for (item1, item2) in zip(&preprocessed_input1, &preprocessed_input2) {
            if item1 != item2 {
                tracing::info!("found: {:<25} != {:<25}", item1, item2);
            }
        }
        let comparison = compare(&preprocessed_input1, &preprocessed_input2);
        assert_eq!(comparison, expected, "All items should match");
    }
}
