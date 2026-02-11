use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::approval::ApprovalManager;
use crate::config::Config;
use crate::context_compaction::ChatCompactor;
use crate::context_loop_detector::LoopDetector;
use crate::context_manager::ContextManager;
use crate::hooks::HookSystem;
use crate::mcp::MCPManager;
use crate::registry::{ToolRegistry, create_default_registry};
use crate::tools_discovery::ToolDiscoveryManager;

pub struct Session {
    pub config: Config,
    pub tool_registry: ToolRegistry,
    pub context_manager: ContextManager,
    pub discovery_manager: ToolDiscoveryManager,
    pub mcp_manager: MCPManager,
    pub chat_compactor: ChatCompactor,
    pub approval_manager: ApprovalManager,
    pub hook_system: HookSystem,
    pub loop_detector: LoopDetector,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub turn_count: u32,
}

impl Session {
    pub fn new(config: Config) -> Self {
        let registry = create_default_registry(&config);
        Self {
            context_manager: ContextManager::new(None),
            discovery_manager: ToolDiscoveryManager::new(),
            mcp_manager: MCPManager::new(),
            chat_compactor: ChatCompactor::new(),
            approval_manager: ApprovalManager::new(config.approval.clone(), config.cwd.clone()),
            hook_system: HookSystem::new(config.clone()),
            loop_detector: LoopDetector::new(),
            session_id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            turn_count: 0,
            tool_registry: registry,
            config,
        }
    }

    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        self.mcp_manager.initialize(&self.config).await?;
        self.mcp_manager.register_tools(&mut self.tool_registry).await;
        self.discovery_manager.discover_all()?;
        self.context_manager = ContextManager::new(self.load_memory());
        Ok(())
    }

    fn load_memory(&self) -> Option<String> {
        let path = crate::config_loader::get_data_dir().join("user_memory.json");
        let Ok(content) = std::fs::read_to_string(path) else {
            return None;
        };
        let Ok(v) = serde_json::from_str::<Value>(&content) else {
            return None;
        };
        let entries = v.get("entries")?.as_object()?;
        if entries.is_empty() {
            return None;
        }
        let mut lines = vec!["User preferences and notes:".to_string()];
        for (k, v) in entries {
            lines.push(format!("- {}: {}", k, v.as_str().unwrap_or("")));
        }
        Some(lines.join("\n"))
    }

    pub fn increment_turn(&mut self) -> u32 {
        self.turn_count += 1;
        self.updated_at = Utc::now();
        self.turn_count
    }

    pub fn get_stats(&self) -> Value {
        serde_json::json!({
            "session_id": self.session_id,
            "created_at": self.created_at.to_rfc3339(),
            "turn_count": self.turn_count,
            "message_count": self.context_manager.message_count(),
            "token_usage": self.context_manager.total_usage,
            "tools_count": self.tool_registry.get_tools().len(),
            "mcp_servers": self.mcp_manager.get_all_servers().len(),
        })
    }
}
