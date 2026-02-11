use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::Config;
use crate::tools::base::{Tool, ToolInvocation, ToolKind, ToolResult};

#[derive(Debug, Deserialize)]
struct SubagentParams {
    goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentDefinition {
    pub name: String,
    pub description: String,
    pub goal_prompt: String,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: u32,
    pub timeout_seconds: f64,
}

pub struct SubagentTool {
    #[allow(dead_code)]
    config: Config,
    definition: SubagentDefinition,
    name: String,
    description: String,
}

impl SubagentTool {
    pub fn new(config: Config, definition: SubagentDefinition) -> Self {
        let name = format!("subagent_{}", definition.name);
        let description = format!("subagent_{}", definition.description);
        Self {
            config,
            definition,
            name,
            description,
        }
    }
}

#[async_trait]
impl Tool for SubagentTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Write
    }

    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"goal":{"type":"string"}},"required":["goal"]})
    }

    fn is_mutating(&self, _params: &Value) -> bool {
        true
    }

    async fn execute(&self, invocation: ToolInvocation) -> Result<ToolResult> {
        let params: SubagentParams = serde_json::from_value(invocation.params)?;
        Ok(ToolResult::success_result(format!(
            "Sub-agent '{}' placeholder run.\nGoal: {}",
            self.definition.name, params.goal
        )))
    }
}

pub fn get_default_subagent_definitions() -> Vec<SubagentDefinition> {
    vec![
        SubagentDefinition {
            name: "codebase_investigator".to_string(),
            description:
                "Investigates the codebase to answer questions about code structure, patterns, and implementations"
                    .to_string(),
            goal_prompt: "Use read_file, grep, glob, and list_dir to investigate. Do not modify any files."
                .to_string(),
            allowed_tools: Some(vec![
                "read_file".to_string(),
                "grep".to_string(),
                "glob".to_string(),
                "list_dir".to_string(),
            ]),
            max_turns: 20,
            timeout_seconds: 600.0,
        },
        SubagentDefinition {
            name: "code_reviewer".to_string(),
            description: "Reviews code changes and provides feedback on quality, bugs, and improvements"
                .to_string(),
            goal_prompt: "Use read_file, list_dir and grep to examine code. Do not modify files."
                .to_string(),
            allowed_tools: Some(vec![
                "read_file".to_string(),
                "grep".to_string(),
                "list_dir".to_string(),
            ]),
            max_turns: 10,
            timeout_seconds: 300.0,
        },
    ]
}

pub fn get_subagent_tools(config: &Config) -> Vec<Arc<dyn Tool>> {
    get_default_subagent_definitions()
        .into_iter()
        .map(|d| Arc::new(SubagentTool::new(config.clone(), d)) as Arc<dyn Tool>)
        .collect()
}
