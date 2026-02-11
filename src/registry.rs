use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use serde_json::Value;

use crate::approval::{ApprovalContext, ApprovalDecision, ApprovalManager};
use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolResult};
use crate::tools::builtin::get_all_builtin_tools;
use crate::tools::subagents::get_subagent_tools;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    mcp_tools: HashMap<String, Arc<dyn Tool>>,
    config: Config,
}

impl ToolRegistry {
    pub fn new(config: Config) -> Self {
        Self {
            tools: HashMap::new(),
            mcp_tools: HashMap::new(),
            config,
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn register_mcp_tool(&mut self, tool: Arc<dyn Tool>) {
        self.mcp_tools.insert(tool.name().to_string(), tool);
    }

    pub fn unregister(&mut self, name: &str) -> bool {
        self.tools.remove(name).is_some()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .get(name)
            .cloned()
            .or_else(|| self.mcp_tools.get(name).cloned())
    }

    pub fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut all = Vec::new();
        for t in self.tools.values() {
            all.push(t.clone());
        }
        for t in self.mcp_tools.values() {
            all.push(t.clone());
        }
        if let Some(allowlist) = &self.config.allowed_tools {
            all.retain(|t| allowlist.contains(&t.name().to_string()));
        }
        all
    }

    pub fn get_schemas(&self) -> Vec<Value> {
        self.get_tools()
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "parameters": t.schema()
                })
            })
            .collect()
    }

    pub async fn invoke(
        &self,
        name: &str,
        params: Value,
        cwd: &Path,
        approval_manager: Option<&ApprovalManager>,
    ) -> Result<ToolResult> {
        let Some(tool) = self.get(name) else {
            return Ok(ToolResult::error_result(format!("Unknown tool: {}", name)));
        };

        let invocation = ToolInvocation {
            params: params.clone(),
            cwd: cwd.to_path_buf(),
        };

        if let Some(approval_manager) = approval_manager {
            if let Some(confirm) = tool.get_confirmation(&invocation).await {
                let ctx = ApprovalContext {
                    tool_name: name.to_string(),
                    is_mutating: tool.is_mutating(&params),
                    affected_paths: confirm.affected_paths,
                    command: confirm.command,
                    is_dangerous: confirm.is_dangerous,
                };
                match approval_manager.check_approval(&ctx) {
                    ApprovalDecision::Rejected => {
                        return Ok(ToolResult::error_result(
                            "Operation rejected by safety policy".to_string(),
                        ));
                    }
                    ApprovalDecision::NeedsConfirmation => {
                        if !approval_manager.request_confirmation(&ctx) {
                            return Ok(ToolResult::error_result(
                                "User rejected the operation".to_string(),
                            ));
                        }
                    }
                    ApprovalDecision::Approved => {}
                }
            }
        }

        tool.execute(invocation).await
    }
}

pub fn create_default_registry(config: &Config) -> ToolRegistry {
    let mut registry = ToolRegistry::new(config.clone());
    for tool in get_all_builtin_tools(config) {
        registry.register(tool);
    }
    for subagent in get_subagent_tools(config) {
        registry.register(subagent);
    }
    registry
}
