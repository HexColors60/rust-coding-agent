use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolKind {
    Read,
    Write,
    Shell,
    Network,
    Memory,
    Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: PathBuf,
    pub old_content: String,
    pub new_content: String,
    pub is_new_file: bool,
    pub is_deletion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: HashMap<String, Value>,
    pub truncated: bool,
    pub diff: Option<FileDiff>,
    pub exit_code: Option<i32>,
}

impl ToolResult {
    pub fn success_result(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
            metadata: HashMap::new(),
            truncated: false,
            diff: None,
            exit_code: None,
        }
    }

    pub fn error_result(error: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
            metadata: HashMap::new(),
            truncated: false,
            diff: None,
            exit_code: None,
        }
    }

    pub fn to_model_output(&self) -> String {
        if self.success {
            return self.output.clone();
        }
        format!(
            "Error: {}\n\nOutput:\n{}",
            self.error.clone().unwrap_or_else(|| "unknown".to_string()),
            self.output
        )
    }
}

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub params: Value,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ToolConfirmation {
    pub tool_name: String,
    pub description: String,
    pub diff: Option<FileDiff>,
    pub affected_paths: Vec<PathBuf>,
    pub command: Option<String>,
    pub is_dangerous: bool,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn kind(&self) -> ToolKind;
    fn schema(&self) -> Value;
    fn is_mutating(&self, _params: &Value) -> bool {
        matches!(
            self.kind(),
            ToolKind::Write | ToolKind::Shell | ToolKind::Network | ToolKind::Memory
        )
    }
    async fn get_confirmation(&self, invocation: &ToolInvocation) -> Option<ToolConfirmation> {
        if !self.is_mutating(&invocation.params) {
            return None;
        }
        Some(ToolConfirmation {
            tool_name: self.name().to_string(),
            description: format!("Execute {}", self.name()),
            diff: None,
            affected_paths: vec![],
            command: None,
            is_dangerous: false,
        })
    }
    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult>;
}
