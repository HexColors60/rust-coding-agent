use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};
use crate::utils_paths::resolve_path;

#[derive(Debug, Deserialize)]
struct ListDirParams {
    #[serde(default = "default_path")]
    path: String,
    #[serde(default)]
    include_hidden: bool,
}

fn default_path() -> String {
    ".".to_string()
}

pub struct ListDirTool {
    #[allow(dead_code)]
    config: Config,
}

impl ListDirTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List contents of a directory."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Read
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"include_hidden":{"type":"boolean"}}})
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: ListDirParams = serde_json::from_value(invocation.params)?;
        let dir = resolve_path(&invocation.cwd, &params.path);
        if !dir.exists() || !dir.is_dir() {
            return Ok(ToolResult::error_result(format!(
                "Directory does not exist: {}",
                dir.display()
            )));
        }
        let mut items = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if !params.include_hidden && name.starts_with('.') {
                continue;
            }
            items.push(if path.is_dir() { format!("{}/", name) } else { name });
        }
        items.sort();
        if items.is_empty() {
            return Ok(ToolResult::success_result("Directory is empty".to_string()));
        }
        Ok(ToolResult::success_result(items.join("\n")))
    }
}
