use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{FileDiff, Tool, ToolConfirmation, ToolInvocation, ToolKind, ToolResult};
use crate::utils_paths::{ensure_parent_directory, resolve_path};

#[derive(Debug, Deserialize)]
struct WriteFileParams {
    path: String,
    content: String,
    #[serde(default = "default_true")]
    create_directories: bool,
}

fn default_true() -> bool {
    true
}

pub struct WriteFileTool {
    #[allow(dead_code)]
    config: Config,
}

impl WriteFileTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"},"create_directories":{"type":"boolean"}},"required":["path","content"]})
    }

    async fn get_confirmation(&self, invocation: &ToolInvocation) -> Option<ToolConfirmation> {
        let params: WriteFileParams = serde_json::from_value(invocation.params.clone()).ok()?;
        let path = resolve_path(&invocation.cwd, &params.path);
        let is_new_file = !path.exists();
        let old_content = std::fs::read_to_string(&path).unwrap_or_default();
        let diff = FileDiff {
            path: path.clone(),
            old_content,
            new_content: params.content,
            is_new_file,
            is_deletion: false,
        };
        Some(ToolConfirmation {
            tool_name: self.name().to_string(),
            description: if is_new_file {
                format!("Created file: {}", path.display())
            } else {
                format!("Updated file: {}", path.display())
            },
            diff: Some(diff),
            affected_paths: vec![path],
            command: None,
            is_dangerous: !is_new_file,
        })
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: WriteFileParams = serde_json::from_value(invocation.params)?;
        let path = resolve_path(&invocation.cwd, &params.path);
        let is_new_file = !path.exists();
        let old_content = std::fs::read_to_string(&path).unwrap_or_default();
        if params.create_directories {
            ensure_parent_directory(&path)?;
        } else if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Ok(ToolResult::error_result(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }
        std::fs::write(&path, &params.content)?;
        let mut result = ToolResult::success_result(format!(
            "{} {} {} lines",
            if is_new_file { "Created" } else { "Updated" },
            path.display(),
            params.content.lines().count()
        ));
        result.diff = Some(FileDiff {
            path,
            old_content,
            new_content: params.content,
            is_new_file,
            is_deletion: false,
        });
        Ok(result)
    }
}
