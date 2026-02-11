use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::mcp::{MCPClient, MCPToolInfo};
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

pub struct MCPTool {
    pub name: String,
    pub client: MCPClient,
    pub tool_info: MCPToolInfo,
}

impl MCPTool {
    pub fn new(client: MCPClient, tool_info: MCPToolInfo, name: String) -> Self {
        Self { name, client, tool_info }
    }
}

#[async_trait]
impl Tool for MCPTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.tool_info.description
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Mcp
    }

    fn schema(&self) -> Value {
        self.tool_info.input_schema.clone()
    }

    fn is_mutating(&self, _params: &Value) -> bool {
        true
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let result = self
            .client
            .call_tool(&self.tool_info.name, invocation.params)
            .await?;
        let output = result
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let is_error = result
            .get("is_error")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_error {
            return Ok(ToolResult::error_result(output));
        }
        Ok(ToolResult::success_result(output))
    }
}
