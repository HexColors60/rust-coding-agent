use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{FileDiff, Tool, ToolConfirmation, ToolInvocation, ToolKind, ToolResult};
use crate::utils_paths::{ensure_parent_directory, resolve_path};

#[derive(Debug, Deserialize)]
struct EditParams {
    path: String,
    #[serde(default)]
    old_string: String,
    new_string: String,
    #[serde(default)]
    replace_all: bool,
}

pub struct EditTool {
    #[allow(dead_code)]
    config: Config,
}

impl EditTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    fn no_match_error(old_string: &str, content: &str, path: &std::path::Path) -> ToolResult {
        let first_term = old_string.split_whitespace().next().unwrap_or_default();
        let mut lines = Vec::new();
        if !first_term.is_empty() {
            for (i, line) in content.lines().enumerate() {
                if line.contains(first_term) {
                    lines.push(format!("Line {}: {}", i + 1, line));
                    if lines.len() >= 3 {
                        break;
                    }
                }
            }
        }
        if lines.is_empty() {
            ToolResult::error_result(format!(
                "old_string not found in {}. Make sure old_string matches exactly.",
                path.display()
            ))
        } else {
            ToolResult::error_result(format!(
                "old_string not found in {}.\nPossible similar lines:\n{}",
                path.display(),
                lines.join("\n")
            ))
        }
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing text."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"old_string":{"type":"string"},"new_string":{"type":"string"},"replace_all":{"type":"boolean"}},"required":["path","new_string"]})
    }

    async fn get_confirmation(&self, invocation: &ToolInvocation) -> Option<ToolConfirmation> {
        let params: EditParams = serde_json::from_value(invocation.params.clone()).ok()?;
        let path = resolve_path(&invocation.cwd, &params.path);
        let old_content = std::fs::read_to_string(&path).unwrap_or_default();
        let new_content = if !path.exists() {
            params.new_string
        } else if params.replace_all {
            old_content.replace(&params.old_string, &params.new_string)
        } else {
            old_content.replacen(&params.old_string, &params.new_string, 1)
        };
        Some(ToolConfirmation {
            tool_name: self.name().to_string(),
            description: format!("Edit file: {}", path.display()),
            diff: Some(FileDiff {
                path: path.clone(),
                old_content,
                new_content,
                is_new_file: !path.exists(),
                is_deletion: false,
            }),
            affected_paths: vec![path],
            command: None,
            is_dangerous: false,
        })
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: EditParams = serde_json::from_value(invocation.params)?;
        let path = resolve_path(&invocation.cwd, &params.path);
        if !path.exists() {
            if !params.old_string.is_empty() {
                return Ok(ToolResult::error_result(format!(
                    "File does not exist: {}. To create a new file, use an empty old_string.",
                    path.display()
                )));
            }
            ensure_parent_directory(&path)?;
            std::fs::write(&path, &params.new_string)?;
            let mut result = ToolResult::success_result(format!(
                "Created {} {} lines",
                path.display(),
                params.new_string.lines().count()
            ));
            result.diff = Some(FileDiff {
                path,
                old_content: String::new(),
                new_content: params.new_string,
                is_new_file: true,
                is_deletion: false,
            });
            return Ok(result);
        }

        let old_content = std::fs::read_to_string(&path)?;
        if params.old_string.is_empty() {
            return Ok(ToolResult::error_result(
                "old_string is empty but file exists. Provide old_string to edit, or use write_file to overwrite."
                    .to_string(),
            ));
        }
        let occurrence_count = old_content.matches(&params.old_string).count();
        if occurrence_count == 0 {
            return Ok(Self::no_match_error(&params.old_string, &old_content, &path));
        }
        if occurrence_count > 1 && !params.replace_all {
            return Ok(ToolResult::error_result(format!(
                "old_string found {} times in {}. Provide more context or set replace_all=true.",
                occurrence_count,
                path.display()
            )));
        }
        let new_content = if params.replace_all {
            old_content.replace(&params.old_string, &params.new_string)
        } else {
            old_content.replacen(&params.old_string, &params.new_string, 1)
        };
        if new_content == old_content {
            return Ok(ToolResult::error_result(
                "No change made - old_string equals new_string".to_string(),
            ));
        }
        std::fs::write(&path, &new_content)?;
        let mut result = ToolResult::success_result(format!("Edited {}", path.display()));
        result.diff = Some(FileDiff {
            path,
            old_content,
            new_content,
            is_new_file: false,
            is_deletion: false,
        });
        Ok(result)
    }
}
