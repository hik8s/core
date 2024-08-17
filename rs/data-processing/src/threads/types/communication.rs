use shared::types::parsedline::ParsedLine;

pub struct ClassificationTask {
    pub parsed_line: ParsedLine,
    pub key: String,
}

pub struct ClassificationResult {
    pub key: String,
    pub success: bool,
    pub id: String,
}
impl ClassificationResult {
    pub fn with_success(task: &ClassificationTask) -> Self {
        ClassificationResult {
            key: task.key.to_owned(),
            success: true,
            id: task.parsed_line.id.to_owned(),
        }
    }
}
