use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::config::Config;

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
    let path = cwd.join("agent.toml");
    if path.exists() { Some(path) } else { None }
}

pub fn get_agent_md_files(cwd: &Path) -> Option<PathBuf> {
    let path = cwd.join("AGENTS.md");
    if path.exists() { Some(path) } else { None }
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

pub fn load_config(cwd: Option<PathBuf>) -> Result<Config> {
    Config::new(cwd)
}
