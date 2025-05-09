use shared::{
    connections::openai::tool_args::format_tool_args, constant::DEFAULT_ITERATION_DEPTH,
    openai_util::Tool,
};

#[derive(Debug)]
pub struct ToolCallTrace {
    pub trace: Vec<Tool>,
    pub user_message: String,
    pub depth: usize,
    pub max_depth: usize,
}

impl ToolCallTrace {
    /// Creates a new ToolCallTrace instance that captures tool calls and the user message
    pub fn new(user_message: String, max_depth: Option<usize>) -> Self {
        Self {
            trace: Vec::new(),
            user_message,
            depth: 0,
            max_depth: max_depth.unwrap_or(DEFAULT_ITERATION_DEPTH),
        }
    }

    /// Adds a tool to the trace
    pub fn add_tool(&mut self, tool: &Tool) {
        self.trace.push(tool.clone());
    }

    pub fn format_request(&self) -> String {
        format!("-> Handling request for prompt: \"{}\"", self.user_message)
    }
    pub fn format_tool_call(&self, tool: &Tool) -> String {
        format!(
            "  {}: {} ({})",
            self.depth,
            tool,
            format_tool_args(&tool.get_args())
        )
    }

    pub fn format_final_message(&self) -> String {
        format!(
            "<- Number of tool calls: {}, iteration depth: {}/{} (actual/max)",
            self.trace.len(),
            self.depth,
            self.max_depth
        )
    }

    pub fn summary(&self) -> String {
        if self.trace.is_empty() {
            return "No tools called".to_string();
        }

        let tool_names: Vec<String> = self.trace.iter().map(|t| t.to_string()).collect();

        format!("Tools: {}", tool_names.join(" → "))
    }
}
