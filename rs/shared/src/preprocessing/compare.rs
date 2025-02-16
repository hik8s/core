pub fn compare(input1: &[String], input2: &[String]) -> Vec<bool> {
    let mut result = Vec::new();
    let max_length = input1.len().max(input2.len());

    for i in 0..max_length {
        result.push(input1.get(i) == input2.get(i));
    }
    result
}

#[cfg(test)]
mod tests {

    use crate::setup_tracing;

    use super::*;
    use rstest::rstest;

    #[rstest]
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

    fn test_compare_strings(#[case] inputs: (&str, &str), #[case] expected: Vec<bool>) {
        setup_tracing(false);
        let (input1, input2) = inputs;
        let input1: Vec<String> = input1.split_whitespace().map(|s| s.to_string()).collect();
        let input2: Vec<String> = input2.split_whitespace().map(|s| s.to_string()).collect();
        let comparison = compare(&input1, &input2);
        assert_eq!(comparison, expected, "All items should match");
    }
}
