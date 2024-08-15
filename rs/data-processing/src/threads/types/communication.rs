use shared::types::parsedline::ParsedLine;

pub struct ClassificationTask {
    pub parsed_line: ParsedLine,
    pub key: String,
}

pub struct ClassificationResult {
    pub key: String,
    pub success: bool,
}
