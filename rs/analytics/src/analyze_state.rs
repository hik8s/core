use redis::Commands;

use shared::connections::redis::connect::RedisConnection;

pub async fn analyze_state(redis: &mut RedisConnection, customer_id: &str) {
    let match_key = format!("{customer_id}:kube-apiserver*");
    let keys: Vec<String> = redis.connection.keys(&match_key).unwrap();
    println!("Found {} keys in Redis:", keys.len());
    for key in keys {
        let _pod_name = key.split(":").last().unwrap();

        // let state = redis.get(customer_id, pod_name).unwrap();

        // println!("{pod_name} state len: {}", state.classes.len());
    }
}
