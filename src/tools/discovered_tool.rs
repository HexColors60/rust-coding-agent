use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    #[serde(default = "default_timeout")]
    pub timeout_sec: u64,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_timeout() -> u64 {
    30
}

pub struct DiscoveredTool {
    spec: DiscoveredToolSpec,
}

impl DiscoveredTool {
    pub fn new(spec: DiscoveredToolSpec) -> Self {
        Self { spec }
    }
}

#[async_trait]
impl Tool for DiscoveredTool {
    fn name(&self) -> &str {
        &self.spec.name
    }

    fn description(&self) -> &str {
        &self.spec.description
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Shell
    }

    fn schema(&self) -> Value {
        self.spec.parameters.clone()
    }

    fn is_mutating(&self, _params: &Value) -> bool {
        true
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd.exe");
            c.arg("/c").arg(&self.spec.command);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("/bin/bash");
            c.arg("-c").arg(&self.spec.command);
            c
        };
        for arg in &self.spec.args {
            cmd.arg(arg);
        }
        cmd.current_dir(
            self.spec
                .cwd
                .clone()
                .unwrap_or_else(|| invocation.cwd.clone()),
        );
        cmd.envs(self.spec.env.clone());
        cmd.env("AI_AGENT_TOOL_NAME", &self.spec.name);
        cmd.env("AI_AGENT_TOOL_PARAMS", invocation.params.to_string());
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            let payload = invocation.params.to_string();
            let _ = stdin.write_all(payload.as_bytes()).await;
            let _ = stdin.shutdown().await;
        }

        let output = timeout(
            Duration::from_secs(self.spec.timeout_sec.max(1)),
            child.wait_with_output(),
        )
        .await;
        let output = match output {
            Ok(v) => v?,
            Err(_) => return Ok(ToolResult::error_result("Discovered tool timed out".to_string())),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let mut result = ToolResult::success_result(stdout.trim().to_string());
        result.success = output.status.success();
        result.exit_code = output.status.code();
        if !stderr.trim().is_empty() {
            result.error = Some(stderr.trim().to_string());
        }
        if !result.success && result.error.is_none() {
            result.error = Some("Discovered tool failed".to_string());
        }
        Ok(result)
    }
}
