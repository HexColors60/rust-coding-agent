use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

use crate::config::{
    ApprovalPolicy, Config, HookConfig, MCPServerConfig, MCPToolConfig, ModelConfig,
    ShellEnvironmentPolicy,
};

#[derive(Debug, Default, Deserialize)]
struct ConfigPartial {
    model_name: Option<String>,
    temperature: Option<f32>,
    approval: Option<String>,
    max_turns: Option<u32>,
    hooks_enabled: Option<bool>,
    allowed_tools: Option<Vec<String>>,
    api_key: Option<String>,
    base_url: Option<String>,
    shell_environment: Option<ShellEnvironmentPolicy>,
    model: Option<ModelConfig>,
    mcp_servers: Option<Vec<MCPServerConfig>>,
    hooks: Option<Vec<HookConfig>>,
}

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rust-coding-agent")
}

pub fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rust-coding-agent")
}

pub fn get_system_config_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

pub fn parse_toml(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Object(Default::default()));
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let toml_value: toml::Value = toml::from_str(&content)?;
    Ok(serde_json::to_value(toml_value)?)
}

pub fn get_project_config(cwd: &Path) -> Option<PathBuf> {
    let candidates = [cwd.join("agent.toml"), cwd.join(".agent").join("config.toml")];
    candidates.into_iter().find(|p| p.exists())
}

pub fn get_agent_md_files(cwd: &Path) -> Option<PathBuf> {
    let path = cwd.join("AGENTS.md");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub fn merge_dicts(
    base: HashMap<String, Value>,
    override_map: HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut out = base;
    for (k, v) in override_map {
        out.insert(k, v);
    }
    out
}

fn parse_partial(path: &Path) -> Result<ConfigPartial> {
    if !path.exists() {
        return Ok(ConfigPartial::default());
    }
    let content = std::fs::read_to_string(path)?;
    let partial: ConfigPartial = toml::from_str(&content)?;
    Ok(partial)
}

fn apply_partial(config: &mut Config, partial: ConfigPartial) {
    if let Some(model_name) = partial.model_name {
        config.model_name = model_name;
    }
    if let Some(temperature) = partial.temperature {
        config.temperature = temperature;
    }
    if let Some(approval) = partial.approval {
        config.approval = match approval.as_str() {
            "on-request" => ApprovalPolicy::OnRequest,
            "auto" => ApprovalPolicy::Auto,
            "never" => ApprovalPolicy::Never,
            "yolo" => ApprovalPolicy::Yolo,
            _ => config.approval.clone(),
        };
    }
    if let Some(max_turns) = partial.max_turns {
        config.max_turns = max_turns;
    }
    if let Some(hooks_enabled) = partial.hooks_enabled {
        config.hooks_enabled = hooks_enabled;
    }
    if let Some(allowed_tools) = partial.allowed_tools {
        config.allowed_tools = Some(allowed_tools);
    }
    if let Some(api_key) = partial.api_key {
        config.api_key = Some(api_key);
    }
    if let Some(base_url) = partial.base_url {
        config.base_url = base_url;
    }
    if let Some(shell_environment) = partial.shell_environment {
        config.shell_environment = shell_environment;
    }
    if let Some(model) = partial.model {
        config.model = model;
    }
    if let Some(mcp_servers) = partial.mcp_servers {
        config.mcp_servers = mcp_servers
            .into_iter()
            .map(|mut s| {
                if s.tools.is_empty() {
                    s.tools = Vec::<MCPToolConfig>::new();
                }
                s
            })
            .collect();
    }
    if let Some(hooks) = partial.hooks {
        config.hooks = hooks;
    }
}

pub fn load_config(cwd: Option<PathBuf>) -> Result<Config> {
    let mut config = Config::new(cwd)?;
    let system_path = get_system_config_path();
    if system_path.exists() {
        let system_partial = parse_partial(&system_path)?;
        apply_partial(&mut config, system_partial);
    }
    if let Some(project_path) = get_project_config(&config.cwd) {
        let project_partial = parse_partial(&project_path)?;
        apply_partial(&mut config, project_partial);
    }
    Ok(config)
}
