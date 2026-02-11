use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};
use crate::utils_paths::{is_binary_file, resolve_path};

#[derive(Debug, Deserialize)]
struct ReadFileParams {
    path: String,
    #[serde(default = "default_offset")]
    offset: usize,
    limit: Option<usize>,
}

fn default_offset() -> usize {
    1
}

pub struct ReadFileTool {
    #[allow(dead_code)]
    config: Config,
}

impl ReadFileTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a text file with line numbers."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Read
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"offset":{"type":"integer"},"limit":{"type":"integer"}},"required":["path"]})
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: ReadFileParams = serde_json::from_value(invocation.params)?;
        let path = resolve_path(&invocation.cwd, &params.path);
        if !path.exists() || !path.is_file() {
            return Ok(ToolResult::error_result(format!("File not found: {}", path.display())));
        }
        if is_binary_file(&path) {
            return Ok(ToolResult::error_result(format!(
                "Cannot read binary file: {}",
                path.display()
            )));
        }
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let start = params.offset.saturating_sub(1);
        let end = params
            .limit
            .map(|v| (start + v).min(lines.len()))
            .unwrap_or(lines.len());
        let mut out = Vec::new();
        for (idx, line) in lines[start..end].iter().enumerate() {
            out.push(format!("{:6}|{}", start + idx + 1, line));
        }
        if out.is_empty() {
            return Ok(ToolResult::success_result("File is empty.".to_string()));
        }
        Ok(ToolResult::success_result(out.join("\n")))
    }
}
