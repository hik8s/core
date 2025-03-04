use shared::{connections::openai::tool_args::format_tool_args, openai_util::Tool};

#[derive(Debug)]
pub struct ToolCallTrace {
    pub trace: Vec<Tool>,
    pub user_message: String,
    pub depth: usize,
}

impl ToolCallTrace {
    /// Creates a new ToolCallTrace instance that captures tool calls and the user message
    pub fn new(user_message: String) -> Self {
        Self {
            trace: Vec::new(),
            user_message,
            depth: 0,
        }
    }

    /// Adds a tool to the trace
    pub fn add_tool(&mut self, tool: &Tool) {
        self.trace.push(tool.clone());
    }

    /// Returns the tool trace as a formatted string for debugging or logging
    pub fn format_trace(&self) -> String {
        let mut result = format!(
            "Number of tool calls: {}, iteration depth {} for: \"{}\"\n",
            self.trace.len(),
            self.depth,
            self.user_message
        );

        for (i, tool) in self.trace.iter().enumerate() {
            result.push_str(&format!(
                "{}. {} ({})\n",
                i + 1,
                tool,
                format_tool_args(&tool.get_args())
            ));
        }

        result
    }

    pub fn summary(&self) -> String {
        if self.trace.is_empty() {
            return "No tools called".to_string();
        }

        let tool_names: Vec<String> = self.trace.iter().map(|t| t.to_string()).collect();

        format!("Tools: {}", tool_names.join(" → "))
    }
}
