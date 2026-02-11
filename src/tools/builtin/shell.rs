use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

use crate::approval::is_dangerous_command;
use crate::config::Config;
use crate::tools::base::{Tool, ToolConfirmation, ToolInvocation, ToolKind, ToolResult};
use crate::utils_paths::resolve_path;

#[derive(Debug, Deserialize)]
struct ShellParams {
    command: String,
    #[serde(default = "default_timeout")]
    timeout: u64,
    cwd: Option<String>,
}

fn default_timeout() -> u64 {
    120
}

pub struct ShellTool {
    #[allow(dead_code)]
    config: Config,
}

impl ShellTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Shell
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"command":{"type":"string"},"timeout":{"type":"integer"},"cwd":{"type":"string"}},"required":["command"]})
    }

    async fn get_confirmation(&self, invocation: &ToolInvocation) -> Option<ToolConfirmation> {
        let params: ShellParams = serde_json::from_value(invocation.params.clone()).ok()?;
        Some(ToolConfirmation {
            tool_name: self.name().to_string(),
            description: format!("Execute: {}", params.command),
            diff: None,
            affected_paths: vec![],
            command: Some(params.command.clone()),
            is_dangerous: is_dangerous_command(&params.command),
        })
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: ShellParams = serde_json::from_value(invocation.params)?;
        if is_dangerous_command(&params.command) {
            return Ok(ToolResult::error_result(format!(
                "Command blocked for safety: {}",
                params.command
            )));
        }

        let cwd = match params.cwd {
            Some(ref p) => resolve_path(&invocation.cwd, p),
            None => invocation.cwd.clone(),
        };
        if !cwd.exists() {
            return Ok(ToolResult::error_result(format!(
                "Working directory doesn't exist: {}",
                cwd.display()
            )));
        }

        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd.exe");
            c.arg("/c").arg(&params.command);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("/bin/bash");
            c.arg("-c").arg(&params.command);
            c
        };

        cmd.current_dir(cwd);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd.spawn()?;
        let output = timeout(Duration::from_secs(params.timeout), child.wait_with_output()).await;
        let output = match output {
            Ok(v) => v?,
            Err(_) => return Ok(ToolResult::error_result(format!("Command timed out after {}s", params.timeout))),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let mut merged = String::new();
        if !stdout.trim().is_empty() {
            merged.push_str(stdout.trim_end());
        }
        if !stderr.trim().is_empty() {
            if !merged.is_empty() {
                merged.push('\n');
            }
            merged.push_str("--- stderr ---\n");
            merged.push_str(stderr.trim_end());
        }
        let mut result = ToolResult::success_result(merged);
        result.success = output.status.success();
        if !output.status.success() {
            result.error = Some(stderr);
        }
        result.exit_code = Some(exit_code);
        Ok(result)
    }
}
