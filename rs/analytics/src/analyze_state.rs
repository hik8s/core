use redis::Commands;

use shared::RedisConnection;

pub async fn analyze_state(redis: &mut RedisConnection, db: &str) {
    let match_key = format!("{}_*", db);
    let keys: Vec<String> = redis.connection.keys(&match_key).unwrap();
    tracing::info!(
        "Found {} keys in Redis that match: {}",
        keys.len(),
        match_key
    );
    for key in keys {
        let state = redis.get(&key).unwrap();
        tracing::info!("{key} state len: {}", state.classes.len());
    }
}
