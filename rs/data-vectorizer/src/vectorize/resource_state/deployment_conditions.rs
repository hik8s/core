use std::collections::HashMap;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentCondition};

use crate::error::DataVectorizationError;

pub fn unique_conditions(conditions: Vec<DeploymentCondition>) -> Vec<DeploymentCondition> {
    let mut map: HashMap<String, DeploymentCondition> = HashMap::new();

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
                if condition.last_update_time > existing.last_update_time {
                    map.insert(hash, condition);
                }
            }
            None => {
                map.insert(hash, condition);
            }
        }
    }

    let mut unique_conditions: Vec<DeploymentCondition> = map.into_values().collect();
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

pub fn get_conditions_len(deployment: &Deployment) -> usize {
    match deployment.status.as_ref() {
        Some(status) => match status.conditions.as_ref() {
            Some(conds) => conds.len(),
            None => 0,
        },
        None => 0,
    }
}

pub fn update_deployment_conditions(
    previous_state: Deployment,
    mut new_state: Deployment,
) -> (Deployment, bool) {
    let mut conditions = get_conditions(&previous_state);
    let previous_num_conditions = conditions.len();
    conditions.extend_from_slice(&get_conditions(&new_state));
    let aggregated_conditions = unique_conditions(conditions);

    if let Some(status) = new_state.status.as_mut() {
        status.conditions = Some(aggregated_conditions);
    }

    let updated_conditions = get_conditions_len(&new_state) > previous_num_conditions;
    (new_state, updated_conditions)
}

pub fn get_deployment_uid(deploy: &Deployment) -> Result<String, DataVectorizationError> {
    deploy
        .metadata
        .uid
        .to_owned()
        .ok_or(DataVectorizationError::MissingField("uid".to_string()))
}

pub fn remove_deploy_managed_fields(deploy: &mut Deployment) {
    deploy.metadata.managed_fields = None;
}
