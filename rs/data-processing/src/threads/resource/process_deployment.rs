use std::collections::HashMap;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentCondition};

pub fn unique_deployment_conditions(
    conditions: Vec<DeploymentCondition>,
) -> Vec<DeploymentCondition> {
    let mut condition_map: HashMap<String, DeploymentCondition> = HashMap::new();

    for condition in conditions {
        let hash = format!(
            "{}:{}:{}:{}",
            condition.message.as_deref().unwrap_or(""),
            condition.reason.as_deref().unwrap_or(""),
            condition.status,
            condition.type_
        );

        match condition_map.get(&hash) {
            Some(existing) => {
                if condition.last_update_time > existing.last_update_time {
                    condition_map.insert(hash, condition);
                }
            }
            None => {
                condition_map.insert(hash, condition);
            }
        }
    }

    let mut unique_conditions: Vec<DeploymentCondition> = condition_map.into_values().collect();
    unique_conditions.sort_by(|a, b| b.last_update_time.cmp(&a.last_update_time));
    unique_conditions
}

pub fn get_conditions(deployment: &Deployment) -> Vec<DeploymentCondition> {
    deployment
        .status
        .as_ref()
        .and_then(|status| status.conditions.to_owned())
        .unwrap_or_default()
}
