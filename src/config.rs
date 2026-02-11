use std::path::PathBuf;

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    pub name: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShellEnvironmentPolicy {
    #[serde(default)]
    pub ignore_default_excludes: bool,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    #[serde(default)]
    pub set_vars: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPServerConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub startup_timeout_sec: u64,
    #[serde(default)]
    pub tools: Vec<MCPToolConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPToolConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalPolicy {
    #[value(name = "on-request")]
    OnRequest,
    #[value(name = "auto")]
    Auto,
    #[value(name = "never")]
    Never,
    #[value(name = "yolo")]
    Yolo,
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum HookTrigger {
    BeforeAgent,
    AfterAgent,
    BeforeTool,
    AfterTool,
    OnError
}

impl Default for HookTrigger {
    fn default() -> Self {
        Self::BeforeTool
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookConfig {
    #[serde(default)]
    pub trigger: HookTrigger,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub timeout_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: ModelConfig,
    pub shell_environment: ShellEnvironmentPolicy,
    pub mcp_servers: Vec<MCPServerConfig>,
    pub hooks: Vec<HookConfig>,
    pub model_name: String,
    pub temperature: f32,
    pub approval: ApprovalPolicy,
    pub cwd: PathBuf,
    pub max_turns: u32,
    pub hooks_enabled: bool,
    pub allowed_tools: Option<Vec<String>>,
    pub api_key: Option<String>,
    pub base_url: String,
}

impl Config {
    pub fn new(cwd: Option<PathBuf>) -> Result<Self> {
        let cwd = match cwd {
            Some(v) => v,
            None => std::env::current_dir()?,
        };

        Ok(Self {
            model: ModelConfig {
                name: "gpt-5".to_string(),
                temperature: 0.2,
            },
            shell_environment: ShellEnvironmentPolicy {
                ignore_default_excludes: false,
                exclude_patterns: vec![
                    "*TOKEN*".to_string(),
                    "*SECRET*".to_string(),
                    "*PASSWORD*".to_string(),
                ],
                set_vars: None,
            },
            mcp_servers: vec![],
            hooks: vec![],
            model_name: "gpt-5".to_string(),
            temperature: 0.2,
            approval: ApprovalPolicy::OnRequest,
            cwd,
            max_turns: 40,
            hooks_enabled: true,
            allowed_tools: None,
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
        })
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if !(0.0..=2.0).contains(&self.temperature) {
            errors.push("temperature must be in [0.0, 2.0]".to_string());
        }
        if self.max_turns == 0 {
            errors.push("max_turns must be > 0".to_string());
        }
        if !self.cwd.exists() {
            errors.push(format!("cwd does not exist: {}", self.cwd.display()));
        }
        errors
    }
}
