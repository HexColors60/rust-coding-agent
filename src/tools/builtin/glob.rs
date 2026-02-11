use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};
use crate::utils_paths::resolve_path;

#[derive(Debug, Deserialize)]
struct GlobParams {
    pattern: String,
    #[serde(default = "default_path")]
    path: String,
}

fn default_path() -> String {
    ".".to_string()
}

pub struct GlobTool {
    #[allow(dead_code)]
    config: Config,
}

impl GlobTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Read
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"]})
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: GlobParams = serde_json::from_value(invocation.params)?;
        let root = resolve_path(&invocation.cwd, &params.path);
        if !root.exists() || !root.is_dir() {
            return Ok(ToolResult::error_result(format!(
                "Directory does not exist: {}",
                root.display()
            )));
        }
        let pattern = root.join(&params.pattern).display().to_string();
        let mut out = Vec::new();
        for item in glob::glob(&pattern)?.flatten() {
            if item.is_file() {
                let rel = item.strip_prefix(&invocation.cwd).unwrap_or(&item);
                out.push(rel.display().to_string());
            }
            if out.len() >= 1000 {
                break;
            }
        }
        Ok(ToolResult::success_result(out.join("\n")))
    }
}
