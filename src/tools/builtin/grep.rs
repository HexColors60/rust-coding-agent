use anyhow::Result;
use async_trait::async_trait;
use regex::RegexBuilder;
use serde::Deserialize;
use serde_json::{Value, json};
use walkdir::WalkDir;

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};
use crate::utils_paths::{is_binary_file, resolve_path};

#[derive(Debug, Deserialize)]
struct GrepParams {
    pattern: String,
    #[serde(default = "default_path")]
    path: String,
    #[serde(default)]
    case_insensitive: bool,
}

fn default_path() -> String {
    ".".to_string()
}

pub struct GrepTool {
    #[allow(dead_code)]
    config: Config,
}

impl GrepTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a regex pattern in file contents."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Read
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"},"case_insensitive":{"type":"boolean"}},"required":["pattern"]})
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: GrepParams = serde_json::from_value(invocation.params)?;
        let root = resolve_path(&invocation.cwd, &params.path);
        if !root.exists() {
            return Ok(ToolResult::error_result(format!(
                "Path does not exist: {}",
                root.display()
            )));
        }
        let regex = RegexBuilder::new(&params.pattern)
            .case_insensitive(params.case_insensitive)
            .build()?;
        let mut out = Vec::new();
        let mut files = Vec::new();
        if root.is_file() {
            files.push(root.clone());
        } else {
            for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
                    if [".git", "node_modules", "__pycache__", ".venv", "venv"].contains(&name) {
                        continue;
                    }
                }
                if p.is_file() && !is_binary_file(p) {
                    files.push(p.to_path_buf());
                    if files.len() >= 500 {
                        break;
                    }
                }
            }
        }
        for file in files {
            let Ok(content) = std::fs::read_to_string(&file) else {
                continue;
            };
            let mut file_hit = false;
            for (i, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    if !file_hit {
                        let rel = file.strip_prefix(&invocation.cwd).unwrap_or(&file);
                        out.push(format!("=== {} ===", rel.display()));
                        file_hit = true;
                    }
                    out.push(format!("{}:{}", i + 1, line));
                }
            }
            if file_hit {
                out.push(String::new());
            }
        }
        if out.is_empty() {
            return Ok(ToolResult::success_result(format!(
                "No matches found for pattern '{}'",
                params.pattern
            )));
        }
        Ok(ToolResult::success_result(out.join("\n")))
    }
}
