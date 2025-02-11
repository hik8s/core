use k8s_openapi::serde::{de::DeserializeOwned, Serialize};
use shared::{
    connections::redis::connect::RedisConnection,
    types::kubeapidata::{KubeApiData, KubeEventType},
};

use crate::threads::error::ProcessThreadError;

pub async fn update_resource_state<T, F>(
    redis: &mut RedisConnection,
    new_state: T,
    data: &mut KubeApiData,
    key: &str,
    update_conditions: F,
) -> Result<bool, ProcessThreadError>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce(T, T) -> (T, bool),
{
    // let new_state: T = serde_json::from_value(data.json.clone())
    //     .map_err(ProcessThreadError::DeserializationError)?;

    let mut requires_vectorization = false;

    match redis
        .get_with_retry::<String>(key)
        .await
        .map_err(ProcessThreadError::RedisGet)?
    {
        None => {
            let json = serde_json::to_string(&new_state)
                .map_err(ProcessThreadError::SerializationError)?;

            redis
                .set_with_retry::<String>(key, &json)
                .await
                .map_err(ProcessThreadError::RedisSet)?;
            requires_vectorization = true;
        }
        Some(json) => {
            let current_state: T =
                serde_json::from_str(&json).map_err(ProcessThreadError::DeserializationError)?;
            let (new_state, is_updated) = update_conditions(current_state, new_state);
            let new_json = serde_json::to_string(&new_state)
                .map_err(ProcessThreadError::SerializationError)?;

            redis
                .set_with_retry::<String>(key, &new_json)
                .await
                .map_err(ProcessThreadError::RedisSet)?;

            if data.event_type == KubeEventType::Delete || is_updated {
                requires_vectorization = true;
            }
            data.json =
                serde_json::to_value(new_state).map_err(ProcessThreadError::SerializationError)?;
        }
    };

    Ok(requires_vectorization)
}
