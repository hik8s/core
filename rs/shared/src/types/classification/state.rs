use redis::{from_redis_value, FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};
use serde::{Deserialize, Serialize};

use super::class::Class;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClassifierState {
    pub classes: Vec<Class>,
}

impl ToRedisArgs for ClassifierState {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let serialized = serde_json::to_string(self).unwrap();
        out.write_arg(serialized.as_bytes());
    }
}

impl FromRedisValue for ClassifierState {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let json_str: String = from_redis_value(v)?;
        Ok(serde_json::from_str::<ClassifierState>(&json_str)?)
    }
}
