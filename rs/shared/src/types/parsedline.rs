use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedLine {
    pub timestamp: i64,
    pub text: String,
    pub id: String,
}

impl ParsedLine {
    pub fn new(ts: &str, text: &str) -> Result<Self, chrono::ParseError> {
        let timestamp = dt_from_ts(ts)?.timestamp_millis();
        let text = text.to_string();

        Ok(ParsedLine {
            timestamp,
            text,
            id: Uuid::new_v4().to_string(),
        })
    }

    pub fn from_line(line: &str) -> Self {
        let mut split = line.splitn(2, 'Z');
        let datetime_str = match split.next() {
            Some(s) => s,
            None => {
                tracing::warn!("Failed to parse datetime from line: {}", line);
                return ParsedLine {
                    timestamp: 0,
                    text: line.to_string(),
                    id: Uuid::new_v4().to_string(),
                };
            }
        };

        let text = split.next().unwrap_or(line);

        ParsedLine::new(datetime_str, text).unwrap_or_else(|_| ParsedLine {
            timestamp: 0,
            text: line.to_string(),
            id: Uuid::new_v4().to_string(),
        })
    }
}

pub fn dt_from_ts(ts: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    match NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S"))
    {
        Ok(dt) => Ok(Utc.from_utc_datetime(&dt)),
        Err(e) => Err(e),
    }
}
