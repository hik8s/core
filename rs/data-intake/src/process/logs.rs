use shared::types::record::log::LogRecord;

pub fn process_chunk(chunk: &str, remainder: &mut String) -> Vec<LogRecord> {
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
        .map(|line| LogRecord::from_line(&line))
        .collect()
}
