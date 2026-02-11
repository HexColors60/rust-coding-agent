use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

#[derive(Debug, Deserialize)]
struct MemoryParams {
    action: String,
    key: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MemoryStore {
    entries: HashMap<String, String>,
}

pub struct MemoryTool {
    #[allow(dead_code)]
    config: Config,
}

impl MemoryTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    fn memory_path() -> PathBuf {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("rust-coding-agent").join("user_memory.json")
    }

    fn load_memory() -> MemoryStore {
        let path = Self::memory_path();
        let Ok(content) = std::fs::read_to_string(path) else {
            return MemoryStore::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    fn save_memory(store: &MemoryStore) -> Result<()> {
        let path = Self::memory_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(store)?)?;
        Ok(())
    }
}

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        "Store and retrieve persistent memory."
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Memory
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"action":{"type":"string"},"key":{"type":"string"},"value":{"type":"string"}},"required":["action"]})
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: MemoryParams = serde_json::from_value(invocation.params)?;
        match params.action.to_lowercase().as_str() {
            "set" => {
                let (Some(key), Some(value)) = (params.key, params.value) else {
                    return Ok(ToolResult::error_result(
                        "`key` and `value` are required for 'set' action".to_string(),
                    ));
                };
                let mut store = Self::load_memory();
                store.entries.insert(key.clone(), value);
                Self::save_memory(&store)?;
                Ok(ToolResult::success_result(format!("Set memory: {}", key)))
            }
            "get" => {
                let Some(key) = params.key else {
                    return Ok(ToolResult::error_result("`key` required for 'get' action".to_string()));
                };
                let store = Self::load_memory();
                match store.entries.get(&key) {
                    Some(v) => Ok(ToolResult::success_result(format!("Memory found: {}: {}", key, v))),
                    None => Ok(ToolResult::success_result(format!("Memory not found: {}", key))),
                }
            }
            "delete" => {
                let Some(key) = params.key else {
                    return Ok(ToolResult::error_result("`key` required for 'delete' action".to_string()));
                };
                let mut store = Self::load_memory();
                store.entries.remove(&key);
                Self::save_memory(&store)?;
                Ok(ToolResult::success_result(format!("Deleted memory: {}", key)))
            }
            "list" => {
                let store = Self::load_memory();
                if store.entries.is_empty() {
                    return Ok(ToolResult::success_result("No memories stored".to_string()));
                }
                let mut lines = vec!["Stored memories:".to_string()];
                let mut pairs: Vec<_> = store.entries.into_iter().collect();
                pairs.sort_by(|a, b| a.0.cmp(&b.0));
                for (k, v) in pairs {
                    lines.push(format!("  {}: {}", k, v));
                }
                Ok(ToolResult::success_result(lines.join("\n")))
            }
            "clear" => {
                let store = Self::load_memory();
                let count = store.entries.len();
                Self::save_memory(&MemoryStore::default())?;
                Ok(ToolResult::success_result(format!("Cleared {} memory entries", count)))
            }
            _ => Ok(ToolResult::error_result(format!("Unknown action: {}", params.action))),
        }
    }
}
