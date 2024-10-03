use std::str::{from_utf8, Utf8Error};

use fluvio::dataplane::record::ConsumerRecord;

pub fn get_record_key(record: &ConsumerRecord) -> Result<String, Utf8Error> {
    let key = record.key().unwrap();
    let key = from_utf8(key)?;
    Ok(key.to_owned())
}
