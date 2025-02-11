use std::collections::HashMap;

use k8s_openapi::api::core::v1::{Pod, PodCondition};

use crate::threads::error::ProcessThreadError;

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
                    // if condition.status == "True" && condition.type_ == "Ready" {
                    //     tracing::info!(
                    //         "time: {:#?} | {}",
                    //         condition
                    //             .last_transition_time
                    //             .clone()
                    //             .unwrap()
                    //             .0
                    //             .to_string(),
                    //         hash
                    //     );
                    // }
                    map.insert(hash, condition.clone());
                }
            }
            None => {
                map.insert(hash, condition.clone());
            }
        }
        // if condition.status == "True" && condition.type_ == "Ready" {
        //     tracing::info!("{:#?}", map.get("::True:Ready"))
        // }
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

pub fn update_pod_conditions(previous_state: Pod, mut new_state: Pod) -> (Pod, bool) {
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

pub fn get_pod_key(pod: &Pod) -> Result<String, ProcessThreadError> {
    let owner_uids = pod.metadata.owner_references.as_ref().map(|refs| {
        refs.iter()
            .map(|owner| owner.uid.as_ref())
            .collect::<Vec<&str>>()
            .join("_")
    });

    if let Some(owner_uids) = owner_uids {
        Ok(owner_uids)
    } else {
        pod.metadata
            .uid
            .to_owned()
            .ok_or(ProcessThreadError::MissingField("uid".to_string()))
    }
}
