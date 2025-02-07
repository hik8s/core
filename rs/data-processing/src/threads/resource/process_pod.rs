use std::collections::HashMap;

use k8s_openapi::api::core::v1::{Pod, PodCondition};

pub fn unique_conditions(conditions: Vec<PodCondition>) -> Vec<PodCondition> {
    let mut map: HashMap<String, PodCondition> = HashMap::new();

    for condition in conditions {
        let hash = format!(
            "{}:{}:{}:{}",
            condition.message.as_deref().unwrap_or(""),
            condition.reason.as_deref().unwrap_or(""),
            condition.status,
            condition.type_
        );

        match map.get(&hash) {
            Some(existing) => {
                if condition.last_transition_time > existing.last_transition_time {
                    map.insert(hash, condition.clone());
                }
            }
            None => {
                map.insert(hash, condition.clone());
            }
        }
    }

    let mut unique_conditions: Vec<PodCondition> = map.into_values().collect();
    unique_conditions.sort_by(|a, b| b.last_transition_time.cmp(&a.last_transition_time));
    unique_conditions
}

pub fn get_conditions(deployment: &Pod) -> Vec<PodCondition> {
    deployment
        .status
        .as_ref()
        .and_then(|status| status.conditions.to_owned())
        .unwrap_or_default()
}

pub fn get_conditions_len(deployment: &Pod) -> usize {
    match deployment.status.as_ref() {
        Some(status) => match status.conditions.as_ref() {
            Some(conds) => conds.len(),
            None => 0,
        },
        None => 0,
    }
}
