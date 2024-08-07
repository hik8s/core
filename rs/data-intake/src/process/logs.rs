use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

pub fn process_chunk(chunk: &str, remainder: &mut String) -> Vec<ParsedLine> {
    // Split the chunk into lines
    let mut log_lines: Vec<String> = chunk.split('\n').map(|line| line.to_string()).collect();

    // If there's a remainder from the previous chunk, prepend it to the first line
    if !remainder.is_empty() && !log_lines.is_empty() {
        log_lines[0] = format!("{}{}", remainder, log_lines[0]);
        remainder.clear();
    }

    // If the chunk doesn't end with a newline, there's a cut-off line
    if !chunk.ends_with('\n') {
        if let Some(last_line) = log_lines.pop() {
            remainder.push_str(&last_line);
        }
    }

    // Filter out empty lines and parse them
    log_lines
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .map(|line| ParsedLine::from_line(&line))
        .collect()
}

pub struct ParsedLine {
    pub dt: DateTime<Utc>,
    pub text: String,
}

impl ParsedLine {
    pub fn new(ts: &str, text: &str) -> Result<Self, chrono::ParseError> {
        let dt = dt_from_ts(ts);
        let text = text.to_string();

        match dt {
            Ok(dt) => Ok(ParsedLine { dt, text }),
            Err(e) => Err(e),
        }
    }
    pub fn from_line(line: &str) -> Self {
        let mut split = line.splitn(2, 'Z');
        let datetime_str = match split.next() {
            Some(s) => s,
            None => {
                tracing::warn!("Failed to parse datetime from line: {}", line);
                return ParsedLine {
                    dt: Utc.with_ymd_and_hms(1970, 0, 0, 0, 0, 0).unwrap(),
                    text: line.to_string(),
                };
            }
        };

        let text = split.next().unwrap_or(line);

        ParsedLine::new(datetime_str, text).ok().unwrap()
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
