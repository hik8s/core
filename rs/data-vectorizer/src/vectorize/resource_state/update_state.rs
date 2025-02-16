use k8s_openapi::serde::{de::DeserializeOwned, Serialize};
use shared::{connections::dbname::DbName, types::kubeapidata::KubeApiData, RedisConnection};

use crate::error::DataVectorizationError;

// TODO: use a wrapped type that implements F, G, H
pub async fn update_resource_state<T, F, G, H>(
    customer_id: &str,
    kind: &str,
    redis: &mut RedisConnection,
    data: &mut KubeApiData,
    update_conditions: F,
    get_uid: G,
    remove_managed_fields: H,
) -> Result<bool, DataVectorizationError>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce(T, T) -> (T, bool),
    G: FnOnce(&T) -> Result<String, DataVectorizationError>,
    H: FnOnce(&mut T),
{
    let mut new_state: T = serde_json::from_value(data.json.clone())
        .map_err(DataVectorizationError::DeserializationError)?;
    remove_managed_fields(&mut new_state);

    let mut requires_vectorization = false;
    let uid = get_uid(&new_state)?;
    let key = redis.key(DbName::Resource, customer_id, Some(kind), &uid);
    match redis
        .get_with_retry::<String>(&key)
        .await
        .map_err(DataVectorizationError::RedisGet)?
    {
        None => {
            let json = serde_json::to_string(&new_state)
                .map_err(DataVectorizationError::SerializationError)?;

            redis
                .set_with_retry::<String>(&key, &json)
                .await
                .map_err(DataVectorizationError::RedisSet)?;
            requires_vectorization = true;
        }
        Some(json) => {
            let current_state: T = serde_json::from_str(&json)
                .map_err(DataVectorizationError::DeserializationError)?;
            let (new_state, is_updated) = update_conditions(current_state, new_state);
            let new_json = serde_json::to_string(&new_state)
                .map_err(DataVectorizationError::SerializationError)?;

            redis
                .set_with_retry::<String>(&key, &new_json)
                .await
                .map_err(DataVectorizationError::RedisSet)?;

            if is_updated {
                requires_vectorization = true;
            }
            data.json = serde_json::to_value(new_state)
                .map_err(DataVectorizationError::SerializationError)?;
        }
    };

    Ok(requires_vectorization)
}
