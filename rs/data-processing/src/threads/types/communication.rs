use shared::types::parsedline::ParsedLine;

pub struct ClassificationTask {
    pub parsed_line: ParsedLine,
    pub key: String,
}

pub struct ClassificationResult {
    pub key: String,
    pub log_id: String,
    pub class_id: String,
}
impl ClassificationResult {
    pub fn new(task: &ClassificationTask, class_id: String) -> Self {
        ClassificationResult {
            key: task.key.to_owned(),
            class_id,
            log_id: task.parsed_line.id.to_owned(),
        }
    }
}
