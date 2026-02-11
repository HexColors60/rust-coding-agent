use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

pub struct MCPTool {
    pub tool_name: String,
    pub tool_description: String,
    pub tool_schema: Value,
}

impl MCPTool {
    pub fn new(tool_name: String, tool_description: String, tool_schema: Value) -> Self {
        Self {
            tool_name,
            tool_description,
            tool_schema,
        }
    }
}

#[async_trait]
impl Tool for MCPTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Mcp
    }

    fn schema(&self) -> Value {
        self.tool_schema.clone()
    }

    async fn execute(&self, _invocation: ToolInvocation) -> Result<ToolResult> {
        Ok(ToolResult::success_result(format!(
            "MCP tool '{}' placeholder execution",
            self.tool_name
        )))
    }
}
