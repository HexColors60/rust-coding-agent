use std::collections::HashMap;
use std::process::Stdio;

use anyhow::Result;
use serde_json::Value;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

use crate::config::{Config, HookConfig, HookTrigger};
use crate::tools::base::ToolResult;

pub struct HookSystem {
    config: Config,
    hooks: Vec<HookConfig>,
}

impl HookSystem {
    pub fn new(config: Config) -> Self {
        let hooks = if config.hooks_enabled {
            config.hooks.iter().filter(|h| h.enabled).cloned().collect()
        } else {
            Vec::new()
        };
        Self { config, hooks }
    }

    async fn run_hook(&self, hook: &HookConfig, env: HashMap<String, String>) -> Result<()> {
        let command = if let Some(cmd) = &hook.command {
            cmd.clone()
        } else if let Some(script) = &hook.script {
            #[cfg(target_os = "windows")]
            {
                format!("powershell -NoProfile -Command {}", script)
            }
            #[cfg(not(target_os = "windows"))]
            {
                format!("/bin/bash -lc {:?}", script)
            }
        } else {
            return Ok(());
        };
        self.run_command(&command, hook.timeout_sec, env).await
    }

    async fn run_command(
        &self,
        command: &str,
        timeout_sec: u64,
        env: HashMap<String, String>,
    ) -> Result<()> {
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd.exe");
            c.arg("/c").arg(command);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("/bin/bash");
            c.arg("-c").arg(command);
            c
        };

        cmd.current_dir(&self.config.cwd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.envs(env);

        let child = cmd.spawn()?;
        let _ = timeout(Duration::from_secs(timeout_sec), child.wait_with_output()).await;
        Ok(())
    }

    fn build_env(
        &self,
        trigger: HookTrigger,
        tool_name: Option<&str>,
        user_message: Option<&str>,
        error: Option<&str>,
    ) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = std::env::vars().collect();
        env.insert(
            "AI_AGENT_TRIGGER".to_string(),
            format!("{:?}", trigger).to_lowercase(),
        );
        env.insert(
            "AI_AGENT_CWD".to_string(),
            self.config.cwd.display().to_string(),
        );
        if let Some(tool_name) = tool_name {
            env.insert("AI_AGENT_TOOL_NAME".to_string(), tool_name.to_string());
        }
        if let Some(user_message) = user_message {
            env.insert("AI_AGENT_USER_MESSAGE".to_string(), user_message.to_string());
        }
        if let Some(error) = error {
            env.insert("AI_AGENT_ERROR".to_string(), error.to_string());
        }
        env
    }

    pub async fn trigger_before_agent(&self, user_message: &str) -> Result<()> {
        let env = self.build_env(HookTrigger::BeforeAgent, None, Some(user_message), None);
        for hook in &self.hooks {
            if matches!(hook.trigger, HookTrigger::BeforeAgent) {
                let _ = self.run_hook(hook, env.clone()).await;
            }
        }
        Ok(())
    }

    pub async fn trigger_after_agent(&self, user_message: &str, agent_response: &str) -> Result<()> {
        let mut env = self.build_env(HookTrigger::AfterAgent, None, Some(user_message), None);
        env.insert("AI_AGENT_RESPONSE".to_string(), agent_response.to_string());
        for hook in &self.hooks {
            if matches!(hook.trigger, HookTrigger::AfterAgent) {
                let _ = self.run_hook(hook, env.clone()).await;
            }
        }
        Ok(())
    }

    pub async fn trigger_before_tool(&self, tool_name: &str, tool_params: &Value) -> Result<()> {
        let mut env = self.build_env(HookTrigger::BeforeTool, Some(tool_name), None, None);
        env.insert("AI_AGENT_TOOL_PARAMS".to_string(), tool_params.to_string());
        for hook in &self.hooks {
            if matches!(hook.trigger, HookTrigger::BeforeTool) {
                let _ = self.run_hook(hook, env.clone()).await;
            }
        }
        Ok(())
    }

    pub async fn trigger_after_tool(
        &self,
        tool_name: &str,
        tool_params: &Value,
        tool_result: &ToolResult,
    ) -> Result<()> {
        let mut env = self.build_env(HookTrigger::AfterTool, Some(tool_name), None, None);
        env.insert("AI_AGENT_TOOL_PARAMS".to_string(), tool_params.to_string());
        env.insert(
            "AI_AGENT_TOOL_RESULT".to_string(),
            tool_result.to_model_output(),
        );
        for hook in &self.hooks {
            if matches!(hook.trigger, HookTrigger::AfterTool) {
                let _ = self.run_hook(hook, env.clone()).await;
            }
        }
        Ok(())
    }

    pub async fn trigger_on_error(&self, error: &str) -> Result<()> {
        let env = self.build_env(HookTrigger::OnError, None, None, Some(error));
        for hook in &self.hooks {
            if matches!(hook.trigger, HookTrigger::OnError) {
                let _ = self.run_hook(hook, env.clone()).await;
            }
        }
        Ok(())
    }

    pub async fn trigger_before_run(&self) -> Result<()> {
        self.trigger_before_agent("").await
    }

    pub async fn trigger_after_run(&self) -> Result<()> {
        self.trigger_after_agent("", "").await
    }
}
