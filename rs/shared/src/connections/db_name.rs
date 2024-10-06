use crate::constant::LOG_PREFIX;

pub fn get_db_name(customer_id: &str) -> String {
    format!("{LOG_PREFIX}_{customer_id}")
}
