pub struct ClassifiedLogRecord {
    pub record_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub class_id: String,
    pub class_name: String,
    pub class_probability: f64,
}
